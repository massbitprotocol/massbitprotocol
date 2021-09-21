use futures::future;
use futures::prelude::*;
use massbit::blockchain::block_stream::BlockWithTriggers;
use massbit::prelude::web3::types::H256;
use massbit::prelude::{
    futures03::{self, compat::Future01CompatExt, FutureExt, StreamExt, TryStreamExt},
    *,
};
use std::collections::{HashMap, HashSet};
use web3::{
    types::{BlockId, Filter, FilterBuilder, Log},
    Web3,
};

use crate::adapter::{
    EthGetLogsFilter, EthereumAdapter as EthereumAdapterTrait, EthereumCallFilter,
    EthereumLogFilter,
};
use crate::chain::BlockFinality;
use crate::transport::Transport;
use crate::trigger::EthereumTrigger;
use crate::types::{LightEthereumBlock, LightEthereumBlockExt};
use crate::TriggerFilter;

lazy_static! {
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
}

#[derive(Clone)]
pub struct EthereumAdapter {
    url_hostname: Arc<String>,
    web3: Arc<Web3<Transport>>,
}

impl CheapClone for EthereumAdapter {
    fn cheap_clone(&self) -> Self {
        Self {
            url_hostname: self.url_hostname.cheap_clone(),
            web3: self.web3.cheap_clone(),
        }
    }
}

impl EthereumAdapter {
    pub async fn new(url: &str, transport: Transport) -> Self {
        let hostname = url::Url::parse(url)
            .unwrap()
            .host_str()
            .unwrap()
            .to_string();

        let web3 = Arc::new(Web3::new(transport));

        EthereumAdapter {
            url_hostname: Arc::new(hostname),
            web3,
        }
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

    fn logs_in_block_range(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        log_filter: EthereumLogFilter,
    ) -> DynTryFuture<'static, Vec<Log>, Error> {
        let eth: Self = self.cheap_clone();
        futures03::stream::iter(
            log_filter
                .eth_get_logs_filters()
                .map(move |filter| eth.cheap_clone().log_stream(from, to, filter)),
        )
        // Real limits on the number of parallel requests are imposed within the adapter.
        .buffered(1000)
        .try_concat()
        .boxed()
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
}

#[async_trait]
impl EthereumAdapterTrait for EthereumAdapter {
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
