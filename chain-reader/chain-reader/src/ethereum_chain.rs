use crate::command::NetworkType;
use crate::Transport;
use crate::{
    grpc_stream::stream_mod::{ChainType, DataType, GenericDataProto},
    CONFIG,
};
use anyhow::Error;
use futures::stream;
use futures::{Future, Stream};
use futures03::{self, compat::Future01CompatExt};
use graph::prelude::tokio::sync::{OwnedSemaphorePermit, Semaphore};
use log::{debug, info, warn};
use massbit_chain_ethereum::data_type::EthereumBlock as Block;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::time::{sleep, timeout, Duration};
use web3;
use web3::transports::Batch;
use web3::{
    types::{
        Block as EthBlock, BlockId, BlockNumber as Web3BlockNumber, Filter, FilterBuilder, Log,
        Transaction, TransactionReceipt, H256,
    },
    Web3,
};

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Ethereum;
const PULLING_INTERVAL: u64 = 200;
const USE_WEBSOCKET: bool = false;
const BLOCK_BATCH_SIZE: u64 = 10;
const RETRY_GET_BLOCK_LIMIT: u32 = 10;
const GET_BLOCK_TIMEOUT_SEC: u64 = 60;

#[derive(Error, Debug)]
pub enum IngestorError {
    /// The Ethereum node does not know about this block for some reason, probably because it
    /// disappeared in a chain reorg.
    #[error("Block data unavailable, block was likely uncled (block hash = {0:?})")]
    BlockUnavailable(H256),

    /// An unexpected error occurred.
    #[error("Ingestor error: {0}")]
    Unknown(Error),
}

async fn wait_for_new_block_http(
    web3_http: &Web3<Transport>,
    got_block_number: &Option<u64>,
) -> u64 {
    loop {
        let block_header = web3_http.eth().block(Web3BlockNumber::Latest.into()).wait();
        if let Ok(Some(block_header)) = block_header {
            let latest_block_number = block_header.number.unwrap().as_u64();
            if let None = got_block_number {
                return latest_block_number;
            } else if latest_block_number > got_block_number.unwrap() {
                return latest_block_number;
            }
        }
        sleep(Duration::from_millis(PULLING_INTERVAL)).await;
        debug!("Wait for new ETHEREUM block at: {:?}", got_block_number);
    }
}

// Todo: add subscribe for get new block
// async fn wait_for_new_block_ws(
//     sub: &mut SubscriptionStream<WebSocket, BlockHeader>,
//     got_block_number: Option<u64>,
// ) -> u64 {
//     let mut latest_block_number = 0;
//     // Wait for new block
//     sub.take(1)
//         .for_each(|x| {
//             println!("Got: {:?}", x);
//             latest_block_number = x.number.unwrap().as_u64();
//             Ok(())
//         })
//         .wait()
//         .unwrap();
//
//     if got_block_number == None || got_block_number.unwrap() < latest_block_number {
//         latest_block_number
//     } else {
//         got_block_number.unwrap()
//     }
// }

pub fn get_logs(
    web3: &Web3<Transport>,
    from: Web3BlockNumber,
    to: Web3BlockNumber,
) -> Result<Vec<Log>, web3::Error> {
    let log_filter: Filter = FilterBuilder::default()
        .from_block(from)
        .to_block(to)
        //.address(filter.contracts.clone())
        //.topics(Some(filter.event_signatures.clone()), None, None, None)
        .build();

    let now = Instant::now();
    // Request logs from client
    let logs = web3
        .eth()
        .logs(log_filter.clone())
        .then(move |result| result)
        .wait();
    let elapsed = now.elapsed();
    debug!("Elapsed getting log: {:.2?}", elapsed);

    logs
}

pub async fn get_receipts(
    block: &EthBlock<Transaction>,
    web3: &Web3<Transport>,
) -> HashMap<H256, TransactionReceipt> {
    let block = block.clone();
    let block_hash = block.hash.unwrap();
    let batching_web3 = Web3::new(Batch::new(web3.transport().clone()));

    let receipt_futures = block
        .transactions
        .iter()
        .map(|tx| {
            let tx_hash = tx.hash;
            batching_web3
                .eth()
                .transaction_receipt(tx_hash)
                .from_err()
                .map_err(IngestorError::Unknown)
                .and_then(move |receipt_opt| {
                    receipt_opt.ok_or_else(move || {
                        // No receipt was returned.
                        //
                        // This can be because the Ethereum node no longer
                        // considers this block to be part of the main chain,
                        // and so the transaction is no longer in the main
                        // chain.  Nothing we can do from here except give up
                        // trying to ingest this block.
                        //
                        // This could also be because the receipt is simply not
                        // available yet.  For that case, we should retry until
                        // it becomes available.
                        IngestorError::BlockUnavailable(block_hash)
                    })
                })
                .and_then(move |receipt| {
                    // Parity nodes seem to return receipts with no block hash
                    // when a transaction is no longer in the main chain, so
                    // treat that case the same as a receipt being absent
                    // entirely.
                    let receipt_block_hash = receipt
                        .block_hash
                        .ok_or_else(|| IngestorError::BlockUnavailable(block_hash))?;

                    // Check if receipt is for the right block
                    if receipt_block_hash != block_hash {
                        // If the receipt came from a different block, then the
                        // Ethereum node no longer considers this block to be
                        // in the main chain.  Nothing we can do from here
                        // except give up trying to ingest this block.
                        // There is no way to get the transaction receipt from
                        // this block.
                        Err(IngestorError::BlockUnavailable(block_hash))
                    } else {
                        Ok((tx_hash, receipt))
                    }
                })
        })
        .collect::<Vec<_>>();

    let my_receipts = batching_web3
        .transport()
        .submit_batch()
        .from_err()
        .map_err(IngestorError::Unknown)
        .and_then(move |_| stream::futures_ordered(receipt_futures).collect())
        .compat()
        .await;
    let receipts = my_receipts
        .unwrap_or(Vec::new())
        .into_iter()
        .collect::<HashMap<H256, TransactionReceipt>>();

    receipts
}

async fn get_block(
    block_number: u64,
    permit: OwnedSemaphorePermit,
    clone_web3: Web3<Transport>,
    clone_version: String,
    chan_clone: broadcast::Sender<GenericDataProto>,
    network: &NetworkType,
) {
    debug!("Before permit block {}", block_number);
    let _permit = permit;
    debug!("After permit block {}", block_number);
    // Get receipts
    let mut block = clone_web3
        .eth()
        .block_with_txs(BlockId::Number(Web3BlockNumber::from(block_number)))
        .wait();
    debug!("After block_with_txs block {}", block_number);
    for i in 0..RETRY_GET_BLOCK_LIMIT {
        if block.is_err() {
            info!(
                "Getting ETHEREUM-{} block {} retry {} times",
                network, block_number, i
            );
            block = clone_web3
                .eth()
                .block_with_txs(BlockId::Number(Web3BlockNumber::from(block_number)))
                .wait();
        } else {
            break;
        }
    }

    if let Ok(Some(block)) = block {
        // Convert to generic
        let block_hash = block.hash.clone().unwrap_or_default().to_string();

        // Get receipts
        info!("Getting ETHEREUM-{} of block: {}", network, block_number);
        let receipts = get_receipts(&block, &clone_web3).await;
        info!(
            "Got ETHEREUM-{} {} receipts of block: {}",
            network,
            receipts.len(),
            block_number
        );
        // Get logs
        let logs = get_logs(
            &clone_web3,
            Web3BlockNumber::from(block_number),
            Web3BlockNumber::from(block_number),
        )
        .unwrap_or(Vec::new());

        let eth_block = Block {
            version: clone_version.clone(),
            timestamp: block.timestamp.as_u64(),
            block,
            receipts,
            logs,
        };

        let generic_data_proto =
            _create_generic_block(block_hash, block_number, &eth_block, clone_version);
        info!(
            "Sending ETHEREUM-{} as generic data: {:?}",
            network, &generic_data_proto.block_number
        );
        let receiver_number = chan_clone.send(generic_data_proto).unwrap();
        debug!(
            "Finished Sending ETHEREUM-{} as generic data: {}",
            network, receiver_number
        );
    } else {
        info!("Got ETHEREUM-{} block error {:?}", network, &block);
    }
    info!("Finish tokio::spawn for getting block: {:?}", &block_number);
}

pub async fn loop_get_block(
    chan: broadcast::Sender<GenericDataProto>,
    got_block_number: &mut Option<u64>,
    network: NetworkType,
) -> Result<(), Box<dyn StdError>> {
    info!("Start get block {:?}", CHAIN_TYPE);
    info!("Init Ethereum adapter");
    let exit = Arc::new(AtomicBool::new(false));
    let config = CONFIG.get_chain_config(&CHAIN_TYPE, &network).unwrap();
    let websocket_url = config.ws.clone();
    let http_url = config.url.clone();

    let (transport_event_loop, transport) = match USE_WEBSOCKET {
        false => Transport::new_rpc(&http_url, Default::default()),
        true => Transport::new_ws(&websocket_url),
    };
    std::mem::forget(transport_event_loop);

    let web3 = Web3::new(transport);

    // Get version
    let version = web3
        .net()
        .version()
        .wait()
        .unwrap_or("Cannot get version".to_string());

    let sem = Arc::new(Semaphore::new(BLOCK_BATCH_SIZE as usize));
    loop {
        if exit.load(Ordering::Relaxed) {
            eprintln!("{}", "exit".to_string());
            break;
        }

        let latest_block_number = wait_for_new_block_http(&web3, got_block_number).await;

        if *got_block_number == None {
            *got_block_number = Some(latest_block_number - 1);
        }

        let pending_block = latest_block_number - got_block_number.unwrap();

        if pending_block >= 1 {
            info!("ETHEREUM-{} pending block: {}", &network, pending_block);
        }

        // Number of getting block
        let getting_block;
        if pending_block > BLOCK_BATCH_SIZE {
            getting_block = BLOCK_BATCH_SIZE;
        } else {
            getting_block = pending_block;
        }

        for block_number in
            (got_block_number.unwrap() + 1)..(got_block_number.unwrap() + 1 + getting_block)
        {
            // Get block
            info!(
                "Getting ETHEREUM-{} block {}, pending block {}",
                &network, block_number, pending_block
            );

            let clone_version = version.clone();
            let chan_clone = chan.clone();
            let clone_web3 = web3.clone();
            // For limit number of spawn task
            debug!(
                "Wait for permit, permits available: {}",
                sem.available_permits()
            );
            let permit = Arc::clone(&sem).acquire_owned().await?;
            debug!(
                "After gave permit, permits available: {}",
                sem.available_permits()
            );
            let clone_network = network.clone();
            tokio::spawn(async move {
                let res = timeout(
                    Duration::from_secs(GET_BLOCK_TIMEOUT_SEC),
                    get_block(
                        block_number,
                        permit,
                        clone_web3,
                        clone_version,
                        chan_clone,
                        &clone_network,
                    ),
                )
                .await;
                if res.is_err() {
                    warn!("get_block timed out at block {}", &block_number);
                }
            });
        }
        *got_block_number = Some(got_block_number.unwrap() + getting_block);
    }
    Ok(())
}

fn _create_generic_block(
    block_hash: String,
    block_number: u64,
    block: &Block,
    version: String,
) -> GenericDataProto {
    let generic_data = GenericDataProto {
        chain_type: CHAIN_TYPE as i32,
        version,
        data_type: DataType::Block as i32,
        block_hash,
        block_number,
        payload: serde_json::to_vec(block).unwrap(),
    };
    generic_data
}
