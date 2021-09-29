use ethabi::{ParamType, Token};
use futures::prelude::*;
use massbit::blockchain::block_stream::BlockWithTriggers;
use massbit::prelude::web3::types::H256;
use massbit::prelude::{
    futures03::{self, compat::Future01CompatExt, FutureExt, StreamExt, TryStreamExt},
    *,
};
use std::collections::{HashMap, HashSet};
use web3::{
    types::{
        Address, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest, Filter,
        FilterBuilder, Log, Trace, TraceFilter, TraceFilterBuilder, H160,
    },
    Web3,
};

use crate::adapter::{
    EthGetLogsFilter, EthereumAdapter as EthereumAdapterTrait, EthereumCallFilter,
    EthereumLogFilter,
};
use crate::chain::BlockFinality;
use crate::transport::Transport;
use crate::trigger::{EthereumBlockTriggerType, EthereumTrigger};
use crate::types::{LightEthereumBlock, LightEthereumBlockExt};
use crate::{EthereumCall, EthereumContractCall, EthereumContractCallError, TriggerFilter};

lazy_static! {
    static ref TRACE_STREAM_STEP_SIZE: BlockNumber = std::env::var("ETHEREUM_TRACE_STREAM_STEP_SIZE")
        .unwrap_or("50".into())
        .parse::<BlockNumber>()
        .expect("invalid trace stream step size");

    /// Maximum range size for `eth.getLogs` requests that dont filter on
    /// contract address, only event signature, and are therefore expensive.
    ///
    /// According to Ethereum node operators, size 500 is reasonable here.
    static ref MAX_EVENT_ONLY_RANGE: BlockNumber = std::env::var("ETHEREUM_MAX_EVENT_ONLY_RANGE")
        .unwrap_or("500".into())
        .parse::<BlockNumber>()
        .expect("invalid number of parallel Ethereum block ranges to scan");

    static ref BLOCK_BATCH_SIZE: usize = std::env::var("ETHEREUM_BLOCK_BATCH_SIZE")
        .unwrap_or("10".into())
        .parse::<usize>()
        .expect("invalid ETHEREUM_BLOCK_BATCH_SIZE env var");

    /// This should not be too large that it causes requests to timeout without us catching it, nor
    /// too small that it causes us to timeout requests that would've succeeded. We've seen
    /// successful `eth_getLogs` requests take over 120 seconds.
    static ref JSON_RPC_TIMEOUT: u64 = std::env::var("ETHEREUM_JSON_RPC_TIMEOUT")
            .unwrap_or("180".into())
            .parse::<u64>()
            .expect("invalid ETHEREUM_JSON_RPC_TIMEOUT env var");

    /// This is used for requests that will not fail the indexer if the limit is reached, but will
    /// simply restart the syncing step, so it can be low. This limit guards against scenarios such
    /// as requesting a block hash that has been reorged.
    static ref REQUEST_RETRIES: usize = std::env::var("ETHEREUM_REQUEST_RETRIES")
            .unwrap_or("10".into())
            .parse::<usize>()
            .expect("invalid ETHEREUM_REQUEST_RETRIES env var");

    /// Gas limit for `eth_call`. The value of 50_000_000 is a protocol-wide parameter so this
    /// should be changed only for debugging purposes and never on an indexer in the network. This
    /// value was chosen because it is the Geth default
    /// https://github.com/ethereum/go-ethereum/blob/e4b687cf462870538743b3218906940ae590e7fd/eth/ethconfig/config.go#L91.
    /// It is not safe to set something higher because Geth will silently override the gas limit
    /// with the default. This means that we do not support indexing against a Geth node with
    /// `RPCGasCap` set below 50 million.
    // See also f0af4ab0-6b7c-4b68-9141-5b79346a5f61.
    static ref ETH_CALL_GAS: u32 = std::env::var("GRAPH_ETH_CALL_GAS")
                                    .map(|s| s.parse::<u32>().expect("invalid GRAPH_ETH_CALL_GAS env var"))
                                    .unwrap_or(50_000_000);

    /// Additional deterministic errors that have not yet been hardcoded. Separated by `;`.
    static ref GETH_ETH_CALL_ERRORS_ENV: Vec<String> = {
        std::env::var("GRAPH_GETH_ETH_CALL_ERRORS")
        .map(|s| s.split(';').filter(|s| s.len() > 0).map(ToOwned::to_owned).collect())
        .unwrap_or(Vec::new())
    };
}

// Deterministic Geth eth_call execution errors. We might need to expand this as
// subgraphs come across other errors. See
// https://github.com/ethereum/go-ethereum/blob/dfeb2f7e8001aef1005a8d5e1605bae1de0b4f12/core/vm/errors.go#L25-L38
const GETH_ETH_CALL_ERRORS: &[&str] = &[
    "execution reverted",
    "invalid jump destination",
    "invalid opcode",
    // Ethereum says 1024 is the stack sizes limit, so this is deterministic.
    "stack limit reached 1024",
    // "out of gas" is commented out because Erigon has not yet bumped the default gas limit to 50
    // million. It can be added through `GETH_ETH_CALL_ERRORS_ENV` if not using Erigon. Once
    // https://github.com/ledgerwatch/erigon/pull/2572 has been released and indexers have updated,
    // this can be uncommented.
    //
    // See f0af4ab0-6b7c-4b68-9141-5b79346a5f61 for why the gas limit is considered deterministic.

    // "out of gas",
];

#[derive(Clone)]
pub struct EthereumAdapter {
    url_hostname: Arc<String>,
    pub web3: Arc<Web3<Transport>>,
    provider: String,
    supports_eip_1898: bool,
}

impl CheapClone for EthereumAdapter {
    fn cheap_clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            url_hostname: self.url_hostname.cheap_clone(),
            web3: self.web3.cheap_clone(),
            supports_eip_1898: self.supports_eip_1898,
        }
    }
}

impl EthereumAdapter {
    pub async fn new(
        provider: String,
        url: &str,
        transport: Transport,
        supports_eip_1898: bool,
    ) -> Self {
        let hostname = url::Url::parse(url)
            .unwrap()
            .host_str()
            .unwrap()
            .to_string();

        let web3 = Arc::new(Web3::new(transport));

        EthereumAdapter {
            provider,
            url_hostname: Arc::new(hostname),
            web3,
            supports_eip_1898,
        }
    }

    async fn traces(
        self,
        from: BlockNumber,
        to: BlockNumber,
        addresses: Vec<H160>,
    ) -> Result<Vec<Trace>, Error> {
        let eth = self.clone();

        retry("trace_filter RPC call")
            .limit(*REQUEST_RETRIES)
            .timeout_secs(*JSON_RPC_TIMEOUT)
            .run(move || {
                let trace_filter: TraceFilter = match addresses.len() {
                    0 => TraceFilterBuilder::default()
                        .from_block(from.into())
                        .to_block(to.into())
                        .build(),
                    _ => TraceFilterBuilder::default()
                        .from_block(from.into())
                        .to_block(to.into())
                        .to_address(addresses.clone())
                        .build(),
                };

                eth.web3
                    .trace()
                    .filter(trace_filter)
                    .map(move |traces| {
                        if traces.len() > 0 {
                            if to == from {
                                debug!("Received {} traces for block {}", traces.len(), to);
                            } else {
                                debug!(
                                    "Received {} traces for blocks [{}, {}]",
                                    traces.len(),
                                    from,
                                    to
                                );
                            }
                        }
                        traces
                    })
                    .from_err()
                    .then(move |result| {
                        if result.is_err() {
                            debug!(
                                "Error querying traces error = {:?} from = {:?} to = {:?}",
                                result, from, to
                            );
                        }
                        result
                    })
                    .compat()
            })
            .map_err(move |e| {
                e.into_inner().unwrap_or_else(move || {
                    anyhow::anyhow!(
                        "Ethereum node took too long to respond to trace_filter \
                         (from block {}, to block {})",
                        from,
                        to
                    )
                })
            })
            .await
    }

    async fn logs_with_sigs(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        filter: Arc<EthGetLogsFilter>,
        too_many_logs_fingerprints: &'static [&'static str],
    ) -> Result<Vec<Log>, TimeoutError<web3::error::Error>> {
        let eth_adapter = self.clone();

        retry("eth_getLogs RPC call")
            .when(move |res: &Result<_, web3::error::Error>| match res {
                Ok(_) => false,
                Err(e) => !too_many_logs_fingerprints
                    .iter()
                    .any(|f| e.to_string().contains(f)),
            })
            .limit(*REQUEST_RETRIES)
            .timeout_secs(*JSON_RPC_TIMEOUT)
            .run(move || {
                // Create a log filter
                let log_filter: Filter = FilterBuilder::default()
                    .from_block(from.into())
                    .to_block(to.into())
                    .address(filter.contracts.clone())
                    .topics(Some(filter.event_signatures.clone()), None, None, None)
                    .build();

                // Request logs from client
                eth_adapter
                    .web3
                    .eth()
                    .logs(log_filter)
                    .then(move |result| result)
                    .compat()
            })
            .await
    }

    fn trace_stream(
        self,
        from: BlockNumber,
        to: BlockNumber,
        addresses: Vec<H160>,
    ) -> impl Stream<Item = Trace, Error = Error> + Send {
        if from > to {
            panic!(
                "Can not produce a call stream on a backwards block range: from = {}, to = {}",
                from, to,
            );
        }

        let step_size = *TRACE_STREAM_STEP_SIZE;

        let eth = self.clone();
        stream::unfold(from, move |start| {
            if start > to {
                return None;
            }
            let end = (start + step_size - 1).min(to);
            let new_start = end + 1;
            if start == end {
                debug!("Requesting traces for block {}", start);
            } else {
                debug!("Requesting traces for blocks [{}, {}]", start, end);
            }
            Some(futures::future::ok((
                eth.clone()
                    .traces(start, end, addresses.clone())
                    .boxed()
                    .compat(),
                new_start,
            )))
        })
        .buffered(*BLOCK_BATCH_SIZE)
        .map(stream::iter_ok)
        .flatten()
    }

    fn log_stream(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        filter: EthGetLogsFilter,
    ) -> DynTryFuture<'static, Vec<Log>, Error> {
        // Codes returned by Ethereum node providers if an eth_getLogs request is too heavy.
        // The first one is for Infura when it hits the log limit, the rest for Alchemy timeouts.
        const TOO_MANY_LOGS_FINGERPRINTS: &[&str] = &[
            "ServerError(-32005)",
            "503 Service Unavailable",
            "ServerError(-32000)",
        ];

        if from > to {
            panic!(
                "cannot produce a log stream on a backwards block range (from={}, to={})",
                from, to
            );
        }

        // Collect all event sigs
        let eth = self.cheap_clone();
        let filter = Arc::new(filter);

        let step = match filter.contracts.is_empty() {
            // `to - from + 1`  blocks will be scanned.
            false => to - from,
            true => (to - from).min(*MAX_EVENT_ONLY_RANGE - 1),
        };

        // Typically this will loop only once and fetch the entire range in one request. But if the
        // node returns an error that signifies the request is to heavy to process, the range will
        // be broken down to smaller steps.
        futures03::stream::try_unfold((from, step), move |(start, step)| {
            let filter = filter.cheap_clone();
            let eth = eth.cheap_clone();

            async move {
                if start > to {
                    return Ok(None);
                }

                let end = (start + step).min(to);
                let res = eth
                    .logs_with_sigs(start, end, filter.cheap_clone(), TOO_MANY_LOGS_FINGERPRINTS)
                    .await;

                match res {
                    Err(e) => {
                        let string_err = e.to_string();

                        // If the step is already 0, the request is too heavy even for a single
                        // block. We hope this never happens, but if it does, make sure to error.
                        if TOO_MANY_LOGS_FINGERPRINTS
                            .iter()
                            .any(|f| string_err.contains(f))
                            && step > 0
                        {
                            // The range size for a request is `step + 1`. So it's ok if the step
                            // goes down to 0, in that case we'll request one block at a time.
                            let new_step = step / 10;
                            Ok(Some((vec![], (start, new_step))))
                        } else {
                            Err(anyhow!("{}", string_err))
                        }
                    }
                    Ok(logs) => Ok(Some((logs, (end + 1, step)))),
                }
            }
        })
        .try_concat()
        .boxed()
    }

    fn call(
        &self,
        contract_address: Address,
        call_data: Bytes,
        block_ptr: BlockPtr,
    ) -> impl Future<Item = Bytes, Error = EthereumContractCallError> + Send {
        let web3 = self.web3.clone();

        // Ganache does not support calls by block hash.
        // See https://github.com/trufflesuite/ganache-cli/issues/745
        let block_id = if !self.supports_eip_1898 {
            BlockId::Number(block_ptr.number.into())
        } else {
            BlockId::Hash(block_ptr.hash_as_h256())
        };

        retry("eth_call RPC call")
            .when(|result| match result {
                Ok(_) | Err(EthereumContractCallError::Revert(_)) => false,
                Err(_) => true,
            })
            .limit(10)
            .timeout_secs(*JSON_RPC_TIMEOUT)
            .run(move || {
                let req = CallRequest {
                    from: None,
                    to: contract_address,
                    gas: Some(web3::types::U256::from(*ETH_CALL_GAS)),
                    gas_price: None,
                    value: None,
                    data: Some(call_data.clone()),
                };
                web3.eth()
                    .call(req, Some(block_id))
                    .then(|result| {
                        // Try to check if the call was reverted. The JSON-RPC response for reverts is
                        // not standardized, so we have ad-hoc checks for each of Geth, Parity and
                        // Ganache.

                        // 0xfe is the "designated bad instruction" of the EVM, and Solidity uses it for
                        // asserts.
                        const PARITY_BAD_INSTRUCTION_FE: &str = "Bad instruction fe";

                        // 0xfd is REVERT, but on some contracts, and only on older blocks,
                        // this happens. Makes sense to consider it a revert as well.
                        const PARITY_BAD_INSTRUCTION_FD: &str = "Bad instruction fd";

                        const PARITY_BAD_JUMP_PREFIX: &str = "Bad jump";
                        const PARITY_STACK_LIMIT_PREFIX: &str = "Out of stack";

                        const GANACHE_VM_EXECUTION_ERROR: i64 = -32000;
                        const GANACHE_REVERT_MESSAGE: &str =
                            "VM Exception while processing transaction: revert";
                        const PARITY_VM_EXECUTION_ERROR: i64 = -32015;
                        const PARITY_REVERT_PREFIX: &str = "Reverted 0x";

                        let mut geth_execution_errors = GETH_ETH_CALL_ERRORS
                            .iter()
                            .map(|s| *s)
                            .chain(GETH_ETH_CALL_ERRORS_ENV.iter().map(|s| s.as_str()));

                        let as_solidity_revert_with_reason = |bytes: &[u8]| {
                            let solidity_revert_function_selector =
                                &tiny_keccak::keccak256(b"Error(string)")[..4];

                            match bytes.len() >= 4
                                && &bytes[..4] == solidity_revert_function_selector
                            {
                                false => None,
                                true => ethabi::decode(&[ParamType::String], &bytes[4..])
                                    .ok()
                                    .and_then(|tokens| tokens[0].clone().to_string()),
                            }
                        };

                        match result {
                            // A successful response.
                            Ok(bytes) => Ok(bytes),

                            // Check for Geth revert, converting to lowercase because some clients
                            // return the same error message as Geth but with capitalization.
                            Err(web3::Error::Rpc(rpc_error))
                                if geth_execution_errors
                                    .any(|e| rpc_error.message.to_lowercase().contains(e)) =>
                            {
                                Err(EthereumContractCallError::Revert(rpc_error.message))
                            }

                            // Check for Parity revert.
                            Err(web3::Error::Rpc(ref rpc_error))
                                if rpc_error.code.code() == PARITY_VM_EXECUTION_ERROR =>
                            {
                                match rpc_error.data.as_ref().and_then(|d| d.as_str()) {
                                    Some(data)
                                        if data.starts_with(PARITY_REVERT_PREFIX)
                                            || data.starts_with(PARITY_BAD_JUMP_PREFIX)
                                            || data.starts_with(PARITY_STACK_LIMIT_PREFIX)
                                            || data == PARITY_BAD_INSTRUCTION_FE
                                            || data == PARITY_BAD_INSTRUCTION_FD =>
                                    {
                                        let reason = if data == PARITY_BAD_INSTRUCTION_FE {
                                            PARITY_BAD_INSTRUCTION_FE.to_owned()
                                        } else {
                                            let payload =
                                                data.trim_start_matches(PARITY_REVERT_PREFIX);
                                            hex::decode(payload)
                                                .ok()
                                                .and_then(|payload| {
                                                    as_solidity_revert_with_reason(&payload)
                                                })
                                                .unwrap_or("no reason".to_owned())
                                        };
                                        Err(EthereumContractCallError::Revert(reason))
                                    }

                                    // The VM execution error was not identified as a revert.
                                    _ => Err(EthereumContractCallError::Web3Error(
                                        web3::Error::Rpc(rpc_error.clone()),
                                    )),
                                }
                            }

                            // Check for Ganache revert.
                            Err(web3::Error::Rpc(ref rpc_error))
                                if rpc_error.code.code() == GANACHE_VM_EXECUTION_ERROR
                                    && rpc_error.message.starts_with(GANACHE_REVERT_MESSAGE) =>
                            {
                                Err(EthereumContractCallError::Revert(rpc_error.message.clone()))
                            }

                            // The error was not identified as a revert.
                            Err(err) => Err(EthereumContractCallError::Web3Error(err)),
                        }
                    })
                    .compat()
            })
            .map_err(|e| e.into_inner().unwrap_or(EthereumContractCallError::Timeout))
            .boxed()
            .compat()
    }

    pub(crate) fn logs_in_block_range(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        log_filter: EthereumLogFilter,
    ) -> DynTryFuture<'static, Vec<Log>, Error> {
        let eth: Self = self.cheap_clone();
        futures03::stream::iter(
            log_filter
                .eth_logs_filters
                .into_iter()
                .map(move |filter| eth.cheap_clone().log_stream(from, to, filter)),
        )
        // Real limits on the number of parallel requests are imposed within the adapter.
        .buffered(1000)
        .try_concat()
        .boxed()
    }

    pub(crate) fn calls_in_block_range<'a>(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        call_filter: &'a EthereumCallFilter,
    ) -> Box<dyn Stream<Item = EthereumCall, Error = Error> + Send + 'a> {
        let eth = self.clone();

        let addresses: Vec<H160> = call_filter
            .contract_addresses_function_signatures
            .iter()
            .filter(|(_addr, (start_block, _fsigs))| start_block <= &to)
            .map(|(addr, (_start_block, _fsigs))| *addr)
            .collect::<HashSet<H160>>()
            .into_iter()
            .collect::<Vec<H160>>();

        if addresses.is_empty() {
            // The filter has no started data sources in the requested range, nothing to do.
            // This prevents an expensive call to `trace_filter` with empty `addresses`.
            return Box::new(stream::empty());
        }

        Box::new(
            eth.trace_stream(from, to, addresses)
                .filter_map(|trace| EthereumCall::try_from_trace(&trace))
                .filter(move |call| {
                    // `trace_filter` can only filter by calls `to` an address and
                    // a block range. Since subgraphs are subscribing to calls
                    // for a specific contract function an additional filter needs
                    // to be applied
                    call_filter.matches(&call)
                }),
        )
    }

    /// Request blocks by hash through JSON-RPC.
    fn load_blocks_rpc(
        &self,
        ids: Vec<H256>,
    ) -> impl Stream<Item = LightEthereumBlock, Error = Error> + Send {
        let web3 = self.web3.clone();

        stream::iter_ok::<_, Error>(ids.into_iter().map(move |hash| {
            let web3 = web3.clone();
            retry(format!("load block {}", hash))
                .limit(*REQUEST_RETRIES)
                .timeout_secs(*JSON_RPC_TIMEOUT)
                .run(move || {
                    web3.eth()
                        .block_with_txs(BlockId::Hash(hash))
                        .from_err::<Error>()
                        .and_then(move |block| {
                            block.ok_or_else(|| {
                                anyhow!("Ethereum node did not find block {:?}", hash)
                            })
                        })
                        .compat()
                })
                .boxed()
                .compat()
                .from_err()
        }))
        .buffered(*BLOCK_BATCH_SIZE)
    }

    /// Request blocks ptrs for numbers through JSON-RPC.
    ///
    /// Reorg safety: If ids are numbers, they must be a final blocks.
    fn load_block_ptrs_rpc(
        &self,
        block_nums: Vec<BlockNumber>,
    ) -> impl Stream<Item = BlockPtr, Error = Error> + Send {
        let web3 = self.web3.clone();

        stream::iter_ok::<_, Error>(block_nums.into_iter().map(move |block_num| {
            let web3 = web3.clone();
            retry(format!("load block ptr {}", block_num))
                .no_limit()
                .timeout_secs(*JSON_RPC_TIMEOUT)
                .run(move || {
                    web3.eth()
                        .block(BlockId::Number(Web3BlockNumber::Number(block_num.into())))
                        .from_err::<Error>()
                        .and_then(move |block| {
                            block.ok_or_else(|| {
                                anyhow!("Ethereum node did not find block {:?}", block_num)
                            })
                        })
                        .compat()
                })
                .boxed()
                .compat()
                .from_err()
        }))
        .buffered(*BLOCK_BATCH_SIZE)
        .map(|b| b.into())
    }

    /// Reorg safety: `to` must be a final block.
    pub(crate) fn block_range_to_ptrs(
        &self,
        from: BlockNumber,
        to: BlockNumber,
    ) -> Box<dyn Future<Item = Vec<BlockPtr>, Error = Error> + Send> {
        // Currently we can't go to the DB for this because there might be duplicate entries for
        // the same block number.
        debug!("Requesting hashes for blocks [{}, {}]", from, to);
        Box::new(self.load_block_ptrs_rpc((from..=to).collect()).collect())
    }

    pub fn contract_call(
        &self,
        call: EthereumContractCall,
    ) -> Box<dyn Future<Item = Vec<Token>, Error = EthereumContractCallError> + Send> {
        // Emit custom error for type mismatches.
        for (token, kind) in call
            .args
            .iter()
            .zip(call.function.inputs.iter().map(|p| &p.kind))
        {
            if !token.type_check(kind) {
                return Box::new(future::err(EthereumContractCallError::TypeError(
                    token.clone(),
                    kind.clone(),
                )));
            }
        }

        // Encode the call parameters according to the ABI
        let call_data = match call.function.encode_input(&call.args) {
            Ok(data) => data,
            Err(e) => return Box::new(future::err(EthereumContractCallError::EncodingError(e))),
        };

        Box::new(
            self.call(
                call.address,
                Bytes(call_data.clone()),
                call.block_ptr.clone(),
            )
            .map(move |result| result.0)
            .and_then(move |output| {
                if output.is_empty() {
                    // We got a `0x` response. For old Geth, this can mean a revert. It can also be
                    // that the contract actually returned an empty response. A view call is meant
                    // to return something, so we treat empty responses the same as reverts.
                    Err(EthereumContractCallError::Revert("empty response".into()))
                } else {
                    // Decode failures are reverts. The reasoning is that if Solidity fails to
                    // decode an argument, that's a revert, so the same goes for the output.
                    call.function.decode_output(&output).map_err(|e| {
                        EthereumContractCallError::Revert(format!("failed to decode output: {}", e))
                    })
                }
            }),
        )
    }
}

#[async_trait]
impl EthereumAdapterTrait for EthereumAdapter {
    fn provider(&self) -> &str {
        &self.provider
    }

    fn block_hash_by_block_number(
        &self,
        block_number: BlockNumber,
    ) -> Box<dyn Future<Item = Option<H256>, Error = Error> + Send> {
        let web3 = self.web3.clone();

        Box::new(
            retry("eth_getBlockByNumber RPC call")
                .no_limit()
                .timeout_secs(*JSON_RPC_TIMEOUT)
                .run(move || {
                    web3.eth()
                        .block(BlockId::Number(block_number.into()))
                        .from_err()
                        .map(|block_opt| block_opt.map(|block| block.hash).flatten())
                        .compat()
                })
                .boxed()
                .compat()
                .map_err(move |e| {
                    e.into_inner().unwrap_or_else(move || {
                        anyhow!(
                            "Ethereum node took too long to return data for block #{}",
                            block_number
                        )
                    })
                }),
        )
    }

    /// Load Ethereum blocks in bulk, returning results as they come back as a Stream.
    fn load_blocks(
        &self,
        block_hashes: HashSet<H256>,
    ) -> Box<dyn Stream<Item = LightEthereumBlock, Error = Error> + Send> {
        let mut blocks: Vec<LightEthereumBlock> = vec![];
        // Return a stream that lazily loads batches of blocks.
        Box::new(
            self.load_blocks_rpc(block_hashes.into_iter().collect())
                .collect()
                .map(move |new_blocks| {
                    blocks.extend(new_blocks);
                    blocks.sort_by_key(|block| block.number);
                    stream::iter_ok(blocks)
                })
                .flatten_stream(),
        )
    }
}

/// Returns blocks with triggers, corresponding to the specified range and filters.
/// If a block contains no triggers, there may be no corresponding item in the stream.
/// However the `to` block will always be present, even if triggers are empty.
///
/// Careful: don't use this function without considering race conditions.
/// Chain reorgs could happen at any time, and could affect the answer received.
/// Generally, it is only safe to use this function with blocks that have received enough
/// confirmations to guarantee no further reorgs, **and** where the Ethereum node is aware of
/// those confirmations.
/// If the Ethereum node is far behind in processing blocks, even old blocks can be subject to
/// reorgs.
/// It is recommended that `to` be far behind the block number of latest block the Ethereum
/// node is aware of.
pub(crate) async fn blocks_with_triggers(
    adapter: Arc<EthereumAdapter>,
    from: BlockNumber,
    to: BlockNumber,
    filter: &TriggerFilter,
) -> Result<Vec<BlockWithTriggers<crate::Chain>>, Error> {
    // Each trigger filter needs to be queried for the same block range
    // and the blocks yielded need to be deduped. If any error occurs
    // while searching for a trigger type, the entire operation fails.
    let eth = adapter.clone();
    let call_filter = EthereumCallFilter::from(filter.block.clone());

    let mut trigger_futs: futures::stream::FuturesUnordered<
        Box<dyn Future<Item = Vec<EthereumTrigger>, Error = Error> + Send>,
    > = futures::stream::FuturesUnordered::new();

    // Scan the block range from triggers to find relevant blocks
    if !filter.log.is_empty() {
        trigger_futs.push(Box::new(
            eth.logs_in_block_range(from, to, filter.log.clone())
                .map_ok(|logs: Vec<Log>| {
                    logs.into_iter()
                        .map(Arc::new)
                        .map(EthereumTrigger::Log)
                        .collect()
                })
                .compat(),
        ))
    }

    if !filter.call.is_empty() {
        trigger_futs.push(Box::new(
            eth.calls_in_block_range(from, to, &filter.call)
                .map(Arc::new)
                .map(EthereumTrigger::Call)
                .collect(),
        ));
    }

    if filter.block.trigger_every_block {
        trigger_futs.push(Box::new(adapter.block_range_to_ptrs(from, to).map(
            move |ptrs| {
                ptrs.into_iter()
                    .map(|ptr| EthereumTrigger::Block(ptr, EthereumBlockTriggerType::Every))
                    .collect()
            },
        )))
    } else if !filter.block.contract_addresses.is_empty() {
        // To determine which blocks include a call to addresses
        // in the block filter, transform the `block_filter` into
        // a `call_filter` and run `blocks_with_calls`
        trigger_futs.push(Box::new(
            eth.calls_in_block_range(from, to, &call_filter)
                .map(|call| {
                    EthereumTrigger::Block(
                        BlockPtr::from(&call),
                        EthereumBlockTriggerType::WithCallTo(call.to),
                    )
                })
                .collect(),
        ));
    }

    let (triggers, to_hash) =
        trigger_futs
            .concat2()
            .join(adapter.clone().block_hash_by_block_number(to).then(
                move |to_hash| match to_hash {
                    Ok(n) => n.ok_or_else(|| anyhow!("Block {} not found in the chain", to)),
                    Err(e) => Err(e),
                },
            ))
            .compat()
            .await?;

    let mut block_hashes: HashSet<H256> =
        triggers.iter().map(EthereumTrigger::block_hash).collect();
    let mut triggers_by_block: HashMap<BlockNumber, Vec<EthereumTrigger>> =
        triggers.into_iter().fold(HashMap::new(), |mut map, t| {
            map.entry(t.block_number()).or_default().push(t);
            map
        });

    debug!("Found {} relevant block(s)", block_hashes.len());

    // Make sure `to` is included, even if empty.
    block_hashes.insert(to_hash);
    triggers_by_block.entry(to).or_insert(Vec::new());

    let mut blocks = adapter
        .load_blocks(block_hashes)
        .and_then(
            move |block| match triggers_by_block.remove(&(block.number() as BlockNumber)) {
                Some(triggers) => Ok(BlockWithTriggers::new(
                    BlockFinality::Final(Arc::new(block)),
                    triggers,
                )),
                None => Err(anyhow!(
                    "block {:?} not found in `triggers_by_block`",
                    block
                )),
            },
        )
        .collect()
        .compat()
        .await?;

    blocks.sort_by_key(|block| block.ptr().number);

    // Sanity check that the returned blocks are in the correct range.
    // Unwrap: `blocks` always includes at least `to`.
    let first = blocks.first().unwrap().ptr().number;
    let last = blocks.last().unwrap().ptr().number;
    if first < from {
        return Err(anyhow!(
            "block {} returned by the Ethereum node is before {}, the first block of the requested range",
            first,
            from,
        ));
    }
    if last > to {
        return Err(anyhow!(
            "block {} returned by the Ethereum node is after {}, the last block of the requested range",
            last,
            to,
        ));
    }

    Ok(blocks)
}
