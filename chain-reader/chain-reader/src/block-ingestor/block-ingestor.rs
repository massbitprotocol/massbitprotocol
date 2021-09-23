use anyhow::Error;
use chain_reader::command;
use chain_reader::Transport;
use chain_reader::{
    grpc_stream::stream_mod::{ChainType, DataType, GenericDataProto},
    CONFIG,
};
use futures::stream;
use futures::{Future, Stream};
use futures03::compat::Future01CompatExt;
use graph::components::ethereum::EthereumBlock as FullEthereumBlock;
use graph::prelude::web3::types::BlockNumber;
use graph::runtime::IndexForAscTypeId::EthereumBlock;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use logger::core::init_logger;
use massbit_chain_ethereum::data_type::EthereumBlock as Block;
use massbit_common::NetworkType;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{sleep, timeout, Duration};
use tonic::Status;
use web3;
use web3::transports::Batch;
use web3::types::U64;
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

fn get_web3(network: &NetworkType) -> Arc<Web3<Transport>> {
    let config = CONFIG.get_chain_config(&CHAIN_TYPE, network).unwrap();
    let websocket_url = config.ws.clone();
    let http_url = config.url.clone();

    let (transport_event_loop, transport) = match USE_WEBSOCKET {
        false => Transport::new_rpc(&http_url, Default::default()),
        true => Transport::new_ws(&websocket_url),
    };
    std::mem::forget(transport_event_loop);
    Arc::new(Web3::new(transport))
}

lazy_static! {
    pub static ref WEB3_ETH: Arc<Web3<Transport>> = get_web3(&"ethereum".to_string());
    pub static ref WEB3_BSC: Arc<Web3<Transport>> = get_web3(&"bsc".to_string());
    pub static ref WEB3_MATIC: Arc<Web3<Transport>> = get_web3(&"matic".to_string());
}

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

// async fn wait_for_new_block_http(
//     web3_http: &Web3<Transport>,
//     got_block_number: &Option<u64>,
// ) -> u64 {
//     loop {
//         let block_header = web3_http.eth().block(Web3BlockNumber::Latest.into()).wait();
//         if let Ok(Some(block_header)) = block_header {
//             let latest_block_number = block_header.number.unwrap().as_u64();
//             if let None = got_block_number {
//                 return latest_block_number;
//             } else if latest_block_number > got_block_number.unwrap() {
//                 return latest_block_number;
//             }
//         }
//         sleep(Duration::from_millis(PULLING_INTERVAL)).await;
//         debug!("Wait for new ETHEREUM block at: {:?}", got_block_number);
//     }
// }
//

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
    //blocks: &Vec<EthBlock<Transaction>>,
    block: &EthBlock<Transaction>,
    web3: &Web3<Transport>,
    //) -> HashMap<H256, TransactionReceipt> {
) -> Result<Vec<TransactionReceipt>, IngestorError> {
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
                        Ok(receipt)
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

    my_receipts
}

pub async fn get_blocks(
    start_block: u64,
    end_block: u64,
    web3: &Web3<Transport>,
) -> Result<Vec<web3::types::Block<Transaction>>, IngestorError> {
    let blocks = (start_block..end_block);
    let batching_web3 = Web3::new(Batch::new(web3.transport().clone()));

    let block_futures = blocks
        .into_iter()
        .map(|number| {
            batching_web3
                .eth()
                .block_with_txs(BlockId::Number(BlockNumber::Number(U64::from(number))))
                .from_err()
                .map_err(IngestorError::Unknown)
                .and_then(move |block_opt| {
                    block_opt.ok_or_else(move || {
                        // Todo: use correct error
                        IngestorError::BlockUnavailable(Default::default())
                    })
                })
                .and_then(move |block| Ok(block))
        })
        .collect::<Vec<_>>();

    let my_blocks = batching_web3
        .transport()
        .submit_batch()
        .from_err()
        .map_err(IngestorError::Unknown)
        .and_then(move |_| stream::futures_ordered(block_futures).collect())
        .compat()
        .await;
    // let blocks = my_blocks
    //     .unwrap_or(Vec::new())
    //     .into_iter()
    //     .collect::<HashMap<u64, web3::types::Block<Transaction>>>();

    my_blocks
}

fn write_full_blocks(
    full_blocks: Vec<FullEthereumBlock>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("full_blocks: {:?}", full_blocks);
    unimplemented!("Write to db");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let res = init_logger(&String::from("chain-reader"));
    println!("Log output: {}", res); // Print log output type

    let network = "matic".to_string();
    // Get version
    let WEB3 = match network.as_str() {
        "bsc" => WEB3_BSC.clone(),
        "matic" => WEB3_MATIC.clone(),
        _ => WEB3_ETH.clone(),
    };
    let now = Instant::now();
    let mut full_blocks = Vec::new();
    let blocks = get_blocks(0, 100, &WEB3).await;
    if let Ok(blocks) = blocks {
        for block in blocks {
            let receipts = get_receipts(&block, &WEB3).await.unwrap_or(Vec::new());
            println!("Got {} receipt of : {:?}", receipts.len(), &block.number);
            let full_block = FullEthereumBlock {
                block: Arc::new(block),
                transaction_receipts: receipts,
            };
            full_blocks.push(full_block);
        }
    } else {
        println!("Cannot get blocks");
    }
    println!("Got Blocks: {:?}", full_blocks.len());
    let elapsed = now.elapsed();
    println!("Run time: {:?}", elapsed);

    write_full_blocks(full_blocks);
    Ok(())
}
