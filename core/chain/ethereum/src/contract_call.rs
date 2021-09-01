use anyhow::{Context, Error};
use futures::future;
use futures::prelude::*;
use log::{debug, info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio_compat_02::FutureExt;

// The graph
use graph::blockchain::types::BlockPtr;
use graph::blockchain::{BlockHash, HostFnCtx};
use graph::cheap_clone::CheapClone;
use graph::log::logger;
use graph::prelude::ethabi::ParamType;
use graph::prelude::{
    error, ethabi,
    ethabi::{Token, Uint},
    retry, tiny_keccak, trace, BlockNumber, EthereumCallCache, Future01CompatExt, MappingABI,
};
use graph::prelude::{lazy_static, tokio};
use graph::runtime::{asc_get, asc_new, AscPtr, HostExportError};
use graph::semver::Version;
use graph_chain_ethereum::runtime::abi::{
    AscUnresolvedContractCall, AscUnresolvedContractCall_0_0_4,
};
use graph_chain_ethereum::runtime::runtime_adapter::UnresolvedContractCall;
use graph_chain_ethereum::{EthereumContractCall, EthereumContractCallError, Transport};
use graph_runtime_wasm::asc_abi::class::{AscEnumArray, EthereumValueKind};

// Web3
use std::time::Instant;
use web3::api::Web3;
use web3::types::{
    Address, Block, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest, Filter,
    FilterBuilder, Log, Transaction, TransactionReceipt, H160, H256,
};

lazy_static! {
    static ref TRACE_STREAM_STEP_SIZE: BlockNumber = std::env::var("ETHEREUM_TRACE_STREAM_STEP_SIZE")
        .unwrap_or("50".into())
        .parse::<BlockNumber>()
        .expect("invalid trace stream step size");

    /// Maximum range size for `eth.getLogs` requests that dont filter on
    /// contract address, only event signature, and are therefore expensive.
    ///
    /// According to Ethereum node operators, size 500 is reasonable here.
    static ref MAX_EVENT_ONLY_RANGE: BlockNumber = std::env::var("GRAPH_ETHEREUM_MAX_EVENT_ONLY_RANGE")
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
    static ref JSON_RPC_TIMEOUT: u64 = std::env::var("GRAPH_ETHEREUM_JSON_RPC_TIMEOUT")
            .unwrap_or("180".into())
            .parse::<u64>()
            .expect("invalid GRAPH_ETHEREUM_JSON_RPC_TIMEOUT env var");


    /// This is used for requests that will not fail the subgraph if the limit is reached, but will
    /// simply restart the syncing step, so it can be low. This limit guards against scenarios such
    /// as requesting a block hash that has been reorged.
    static ref REQUEST_RETRIES: usize = std::env::var("GRAPH_ETHEREUM_REQUEST_RETRIES")
            .unwrap_or("10".into())
            .parse::<usize>()
            .expect("invalid GRAPH_ETHEREUM_REQUEST_RETRIES env var");

    /// Gas limit for `eth_call`. The value of 25_000_000 is a protocol-wide parameter so this
    /// should be changed only for debugging purposes and never on an indexer in the network. The
    /// value of 25_000_000 was chosen because it is the Geth default
    /// https://github.com/ethereum/go-ethereum/blob/54c0d573d75ab9baa239db3f071d6cb4d1ec6aad/eth/ethconfig/config.go#L86.
    /// It is not safe to set something higher because Geth will silently override the gas limit
    /// with the default. This means that we do not support indexing against a Geth node with
    /// `RPCGasCap` set below 25 million.
    // See also f0af4ab0-6b7c-4b68-9141-5b79346a5f61.
    static ref ETH_CALL_GAS: u32 = std::env::var("GRAPH_ETH_CALL_GAS")
                                    .map(|s| s.parse::<u32>().expect("invalid GRAPH_ETH_CALL_GAS env var"))
                                    .unwrap_or(25_000_000);
}

#[derive(Clone)]
pub struct SimpleEthereumAdapter {
    pub url_hostname: Arc<String>,
    pub provider: String,
    pub web3: Arc<Web3<Transport>>,
    pub supports_eip_1898: bool,
}

pub struct SimpleEthereumCallCache {
    pub map: HashMap<(ethabi::Address, Vec<u8>, BlockPtr), Vec<u8>>,
}

pub trait SimpleEthereumCallCacheTrait: Send + Sync + 'static {
    /// Cached return value.
    fn get_call(
        &self,
        contract_address: ethabi::Address,
        encoded_call: &[u8],
        block: BlockPtr,
    ) -> Result<Option<Vec<u8>>, Error>;

    // Add entry to the cache.
    fn set_call(
        &mut self,
        contract_address: ethabi::Address,
        encoded_call: &[u8],
        block: BlockPtr,
        return_value: &[u8],
    ) -> Result<(), Error>;
}

impl SimpleEthereumCallCacheTrait for SimpleEthereumCallCache {
    // Cached return value.
    fn get_call(
        &self,
        contract_address: ethabi::Address,
        encoded_call: &[u8],
        block: BlockPtr,
    ) -> Result<Option<Vec<u8>>, Error> {
        let result = self
            .map
            .get(&(contract_address, encoded_call.to_vec(), block));
        let result = match result {
            Some(result) => Some(result.clone()),
            _ => None,
        };

        return Ok(result);
    }

    // Add entry to the cache.
    fn set_call(
        &mut self,
        contract_address: ethabi::Address,
        encoded_call: &[u8],
        block: BlockPtr,
        return_value: &[u8],
    ) -> Result<(), Error> {
        self.map.insert(
            (contract_address, encoded_call.to_vec(), block),
            return_value.to_vec(),
        );
        Ok(())
    }
}

impl SimpleEthereumAdapter {
    fn contract_call(
        &self,
        call: EthereumContractCall,
        cache: Arc<Mutex<dyn SimpleEthereumCallCacheTrait>>,
    ) -> Box<dyn Future<Item = Vec<Token>, Error = EthereumContractCallError> + Send> {
        //Todo: clean logger
        let logger = logger(true);
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
        debug!("call_data: {:?}", &call_data);
        trace!(logger, "eth_call";
            "address" => hex::encode(&call.address),
            "data" => hex::encode(&call_data)
        );

        let guard = cache.lock().unwrap();
        let cache_result = guard
            .get_call(call.address, &call_data, call.block_ptr.clone())
            .map_err(|e| error!(logger, "call cache get error"; "error" => e.to_string()))
            .ok()
            .flatten();
        drop(guard);
        debug!("cache_result: {:?}", &cache_result);
        // Check if we have it cached, if not do the call and cache.
        Box::new(
            match cache_result {
                Some(result) => {
                    Box::new(future::ok(result)) as Box<dyn Future<Item = _, Error = _> + Send>
                }
                None => {
                    let cache = cache.clone();
                    let call = call.clone();
                    let logger = logger.clone();
                    Box::new(
                        self.call(
                            call.address,
                            Bytes(call_data.clone()),
                            call.block_ptr.clone(),
                        )
                        .map(move |result| {
                            debug!("call result: {:?}", &result);
                            // Don't block handler execution on writing to the cache.
                            let for_cache = result.0.clone();
                            // Todo: Avoid block handler execution on writing to the cache. Now use on-mem db so it is not a problem.
                            //let _ = graph::spawn_blocking_allow_panic(move || {
                            debug!("Start writing cache");
                            cache
                                .lock()
                                .unwrap()
                                .set_call(call.address, &call_data, call.block_ptr, &for_cache)
                                .map_err(|e| {
                                    error!(logger, "call cache set error";
                                                   "error" => e.to_string())
                                });
                            debug!("Finished writing cache!");
                            //                            });
                            result.0
                        }),
                    )
                }
            }
            // Decode the return values according to the ABI
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

    fn call(
        &self,
        contract_address: Address,
        call_data: Bytes,
        block_ptr: BlockPtr,
    ) -> impl Future<Item = Bytes, Error = EthereumContractCallError> + Send {
        //Todo: clean logger
        let logger = logger(true);
        let web3 = self.web3.clone();
        // Ganache does not support calls by block hash.
        // See https://github.com/trufflesuite/ganache-cli/issues/745
        let block_id = if !self.supports_eip_1898 {
            BlockId::Number(block_ptr.number.into())
        } else {
            BlockId::Hash(block_ptr.hash_as_h256())
        };
        // Todo: add retry code
        // retry("eth_call RPC call", &logger)
        //     .when(|result| match result {
        //         Ok(_) | Err(EthereumContractCallError::Revert(_)) => false,
        //         Err(_) => true,
        //     })
        //     .limit(10)
        //     .timeout_secs(*JSON_RPC_TIMEOUT)
        //     .run(move || {
        let req = CallRequest {
            from: None,
            to: contract_address,
            gas: Some(web3::types::U256::from(*ETH_CALL_GAS)),
            gas_price: None,
            value: None,
            data: Some(call_data.clone()),
        };
        debug!("req: {:?}", &req);
        web3.eth().call(req, Some(block_id)).then(|result| {
            debug!("web3.eth().call result: {:?}", &result);
            // Try to check if the call was reverted. The JSON-RPC response for reverts is
            // not standardized, so we have ad-hoc checks for each of Geth, Parity and
            // Ganache.eq

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

            // Deterministic Geth execution errors. We might need to expand this as
            // subgraphs come across other errors. See
            // https://github.com/ethereum/go-ethereum/blob/cd57d5cd38ef692de8fbedaa56598b4e9fbfbabc/core/vm/errors.go
            const GETH_EXECUTION_ERRORS: &[&str] = &[
                "execution reverted",
                "invalid jump destination",
                "invalid opcode",
                // Ethereum says 1024 is the stack sizes limit, so this is deterministic.
                "stack limit reached 1024",
                // See f0af4ab0-6b7c-4b68-9141-5b79346a5f61 for why the gas limit is considered deterministic.
                "out of gas",
            ];

            let as_solidity_revert_with_reason = |bytes: &[u8]| {
                let solidity_revert_function_selector =
                    &tiny_keccak::keccak256(b"Error(string)")[..4];

                match bytes.len() >= 4 && &bytes[..4] == solidity_revert_function_selector {
                    false => None,
                    true => ethabi::decode(&[ParamType::String], &bytes[4..])
                        .ok()
                        .and_then(|tokens| tokens[0].clone().to_string()),
                }
            };

            match result {
                // A successful response.
                Ok(bytes) => Ok(bytes),

                // Check for Geth revert.
                Err(web3::Error::Rpc(rpc_error))
                    if GETH_EXECUTION_ERRORS
                        .iter()
                        .any(|e| rpc_error.message.contains(e)) =>
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
                                let payload = data.trim_start_matches(PARITY_REVERT_PREFIX);
                                hex::decode(payload)
                                    .ok()
                                    .and_then(|payload| as_solidity_revert_with_reason(&payload))
                                    .unwrap_or("no reason".to_owned())
                            };
                            Err(EthereumContractCallError::Revert(reason))
                        }

                        // The VM execution error was not identified as a revert.
                        _ => Err(EthereumContractCallError::Web3Error(web3::Error::Rpc(
                            rpc_error.clone(),
                        ))),
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
        // })
        // .map_err(|e| e.into_inner().unwrap_or(EthereumContractCallError::Timeout))
    }
}

pub fn ethereum_call(
    eth_adapter: &SimpleEthereumAdapter,
    call_cache: Arc<Mutex<dyn SimpleEthereumCallCacheTrait>>,
    ctx: HostFnCtx<'_>,
    wasm_ptr: u32,
    abis: &[Arc<MappingABI>],
) -> Result<AscEnumArray<EthereumValueKind>, HostExportError> {
    // For apiVersion >= 0.0.4 the call passed from the mapping includes the
    // function signature; subgraphs using an apiVersion < 0.0.4 don't pass
    // the signature along with the call.
    let call: UnresolvedContractCall = if ctx.heap.api_version() >= Version::new(0, 0, 4) {
        asc_get::<_, AscUnresolvedContractCall_0_0_4, _>(ctx.heap, wasm_ptr.into())?
    } else {
        asc_get::<_, AscUnresolvedContractCall, _>(ctx.heap, wasm_ptr.into())?
    };
    let function_name = call.function_name.clone();
    let result = eth_call(eth_adapter, call_cache, &ctx.block_ptr, call, abis)?;
    match result {
        Some(tokens) => Ok(asc_new(ctx.heap, tokens.as_slice())?),
        None => match function_name.as_str() {
            "totalSupply" | "decimals" | "balanceOf" => Ok(asc_new(
                ctx.heap,
                vec![Token::Uint(Uint::from(0_u128))].as_slice(),
            )?),
            _ => Ok(AscPtr::null()),
        },
    }
}

/// Returns `Ok(None)` if the call was reverted.
fn eth_call(
    eth_adapter: &SimpleEthereumAdapter,
    call_cache: Arc<Mutex<dyn SimpleEthereumCallCacheTrait>>,
    block_ptr: &BlockPtr,
    unresolved_call: UnresolvedContractCall,
    abis: &[Arc<MappingABI>],
) -> Result<Option<Vec<Token>>, HostExportError> {
    let start_time = Instant::now();

    // Obtain the path to the contract ABI
    let contract = abis
        .iter()
        .find(|abi| abi.name == unresolved_call.contract_name)
        .with_context(|| {
            format!(
                "Could not find ABI for contract \"{}\", try adding it to the 'abis' section \
                         of the subgraph manifest",
                unresolved_call.contract_name
            )
        })?
        .contract
        .clone();
    debug!("contract: {:?}", &contract);
    let function = match unresolved_call.function_signature {
        // Behavior for apiVersion < 0.0.4: look up function by name; for overloaded
        // functions this always picks the same overloaded variant, which is incorrect
        // and may lead to encoding/decoding errors
        None => contract
            .function(unresolved_call.function_name.as_str())
            .with_context(|| {
                format!(
                    "Unknown function \"{}::{}\" called from WASM runtime",
                    unresolved_call.contract_name, unresolved_call.function_name
                )
            })?,

        // Behavior for apiVersion >= 0.0.04: look up function by signature of
        // the form `functionName(uint256,string) returns (bytes32,string)`; this
        // correctly picks the correct variant of an overloaded function
        Some(ref function_signature) => contract
            .functions_by_name(unresolved_call.function_name.as_str())
            .with_context(|| {
                format!(
                    "Unknown function \"{}::{}\" called from WASM runtime",
                    unresolved_call.contract_name, unresolved_call.function_name
                )
            })?
            .iter()
            .find(|f| function_signature == &f.signature())
            .with_context(|| {
                format!(
                    "Unknown function \"{}::{}\" with signature `{}` \
                             called from WASM runtime",
                    unresolved_call.contract_name,
                    unresolved_call.function_name,
                    function_signature,
                )
            })?,
    };
    debug!("function: {:?}", &function);

    let call = EthereumContractCall {
        address: unresolved_call.contract_address.clone(),
        block_ptr: block_ptr.cheap_clone(),
        function: function.clone(),
        args: unresolved_call.function_args.clone(),
    };

    debug!("call: {:?}", &call);
    // Run Ethereum call in tokio runtime
    let logger1 = logger.clone();
    //let call_cache = call_cache.clone();
    let mut result_contract_call = eth_adapter
        .contract_call(call.clone(), call_cache.clone())
        .wait();
    let mut loop_counter = 0;
    info!("result_contract_call: {:?}", &result_contract_call);
    loop {
        let mut retry = match result_contract_call {
            Ok(_) | Err(EthereumContractCallError::Revert(_)) => false,
            Err(_) => true,
        };
        if retry && loop_counter < 10 {
            loop_counter = loop_counter + 1;
            thread::sleep(Duration::from_millis(100));
            result_contract_call = eth_adapter
                .contract_call(call.clone(), call_cache.clone())
                .wait();
            info!(
                "result_contract_call at retry {}-th: {:?}",
                loop_counter, &result_contract_call
            );
        } else {
            break;
        }
    }
    let result =
        //match graph::block_on(eth_adapter.contract_call(call, call_cache).compat())
        match result_contract_call
        {
        Ok(tokens) => {
            info!("Contract Call result {:?}", &tokens);
            Ok(Some(tokens))
        },
        Err(EthereumContractCallError::Revert(reason)) => {
            debug!("Contract call reverted, reason {}", reason);
            Ok(None)
        }

        // Any error reported by the Ethereum node could be due to the block no longer being on
        // the main chain. This is very unespecific but we don't want to risk failing a
        // subgraph due to a transient error such as a reorg.
        Err(EthereumContractCallError::Web3Error(e)) => Err(HostExportError::PossibleReorg(anyhow::anyhow!(
                    "Ethereum node returned an error when calling function \"{}\" of contract \"{}\": {}",
                    unresolved_call.function_name,
                    unresolved_call.contract_name,
                    e
                ))),

        // Also retry on timeouts.
        Err(EthereumContractCallError::Timeout) => Err(HostExportError::PossibleReorg(anyhow::anyhow!(
                    "Ethereum node did not respond when calling function \"{}\" of contract \"{}\"",
                    unresolved_call.function_name,
                    unresolved_call.contract_name,
                ))),

        Err(e) => Err(HostExportError::Unknown(anyhow::anyhow!(
                    "Failed to call function \"{}\" of contract \"{}\": {}",
                    unresolved_call.function_name,
                    unresolved_call.contract_name,
                    e
                ))),
    };
    log::info!("Contract call finished: address: {:?}, contract: {}, function: {}, function_signature: {:?}, result: {:?}, time: {:?}",
             &unresolved_call.contract_address,
             &unresolved_call.contract_name,
             &unresolved_call.function_name,
             &unresolved_call.function_signature,
             &result,
             start_time.elapsed());

    result
}
