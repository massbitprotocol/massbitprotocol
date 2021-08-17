use crate::Transport;
use crate::{
    grpc_stream::stream_mod::{ChainType, DataType, GenericDataProto},
    CONFIG,
};
use futures::stream;
use futures::{Future, Stream};
use futures03::{self, compat::Future01CompatExt};
use log::info;
use massbit_chain_ethereum::data_type::EthereumBlock as Block;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use web3;
use web3::transports::Batch;
use web3::{
    futures::future,
    types::{
        Address, Block as EthBlock, BlockId, BlockNumber as Web3BlockNumber, Bytes, CallRequest,
        Filter, FilterBuilder, Log, Transaction, TransactionReceipt, H256,
    },
    Web3,
};

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Ethereum;
const PULLING_INTERVAL: u64 = 200;
const USE_WEBSOCKET: bool = true;

fn fix_one_thread_not_receive(chan: &broadcast::Sender<GenericDataProto>) {
    // Todo: More clean solution for broadcast channel
    let mut rx = chan.subscribe();
    tokio::spawn(async move {
        loop {
            let _ = rx.recv().await;
        }
    });
}

async fn wait_for_new_block_http(
    web3_http: &Web3<Transport>,
    got_block_number: Option<u64>,
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

pub async fn get_receipts(
    block: &EthBlock<Transaction>,
    web3: &Web3<Transport>,
) -> HashMap<H256, TransactionReceipt> {
    let block = block.clone();
    let batching_web3 = Web3::new(Batch::new(web3.transport().clone()));

    let receipt_futures = block
        .transactions
        .iter()
        .map(|tx| {
            let tx_hash = tx.hash;
            // Todo: add check error (as in commented code)
            batching_web3
                .eth()
                .transaction_receipt(tx_hash)
                // .from_err()
                // .map_err(MyIngestorError::Unknown)
                .and_then(move |receipt_opt| {
                    Ok(receipt_opt.unwrap())
                    //     .ok_or_else(move || {
                    //     // No receipt was returned.
                    //     //
                    //     // This can be because the Ethereum node no longer
                    //     // considers this block to be part of the main chain,
                    //     // and so the transaction is no longer in the main
                    //     // chain.  Nothing we can do from here except give up
                    //     // trying to ingest this block.
                    //     //
                    //     // This could also be because the receipt is simply not
                    //     // available yet.  For that case, we should retry until
                    //     // it becomes available.
                    //     MyIngestorError::BlockUnavailable(block_hash)
                    // })
                })
                .and_then(move |receipt| {
                    // Parity nodes seem to return receipts with no block hash
                    // when a transaction is no longer in the main chain, so
                    // treat that case the same as a receipt being absent
                    // entirely.
                    // let receipt_block_hash = receipt
                    //     .block_hash
                    //     .ok_or_else(|| MyIngestorError::BlockUnavailable(block_hash))?;
                    //
                    // // Check if receipt is for the right block
                    // if receipt_block_hash != block_hash {
                    //     // If the receipt came from a different block, then the
                    //     // Ethereum node no longer considers this block to be
                    //     // in the main chain.  Nothing we can do from here
                    //     // except give up trying to ingest this block.
                    //     // There is no way to get the transaction receipt from
                    //     // this block.
                    //     Err(MyIngestorError::BlockUnavailable(block_hash))
                    // } else {
                    //     Ok((tx_hash, receipt))
                    // }
                    Ok((tx_hash, receipt))
                })
        })
        .collect::<Vec<_>>();

    let my_receipts = batching_web3
        .transport()
        .submit_batch()
        // .from_err()
        // .map_err(MyIngestorError::Unknown)
        .and_then(move |_| stream::futures_ordered(receipt_futures).collect())
        .compat()
        .await;
    let receipts = my_receipts
        .unwrap()
        .into_iter()
        .collect::<HashMap<H256, TransactionReceipt>>();

    receipts
}

pub async fn loop_get_block(chan: broadcast::Sender<GenericDataProto>) {
    info!("Start get block {:?}", CHAIN_TYPE);
    info!("Init Ethereum adapter");
    let exit = Arc::new(AtomicBool::new(false));
    let config = CONFIG.chains.get(&CHAIN_TYPE).unwrap();
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

    fix_one_thread_not_receive(&chan);
    let mut got_block_number = None;
    loop {
        if exit.load(Ordering::Relaxed) {
            eprintln!("{}", "exit".to_string());
            break;
        }

        let latest_block_number = wait_for_new_block_http(&web3, got_block_number).await;

        if got_block_number == None {
            got_block_number = Some(latest_block_number - 1);
        }

        if latest_block_number - got_block_number.unwrap() >= 1 {
            info!(
                "ETHEREUM pending block: {}",
                latest_block_number - got_block_number.unwrap()
            );
        }

        for block_number in (got_block_number.unwrap() + 1)..(latest_block_number + 1) {
            let clone_version = version.clone();
            let chan_clone = chan.clone();
            let clone_web3 = web3.clone();
            tokio::spawn(async move {
                // Get block
                info!("Getting ETHEREUM block {}", block_number);
                // Get receipts
                let block = clone_web3
                    .eth()
                    .block_with_txs(BlockId::Number(Web3BlockNumber::from(block_number)))
                    .wait();

                if let Ok(Some(block)) = block {
                    //println!("Got ETHEREUM Block {:?}",block);
                    // Convert to generic
                    let block_hash = block.hash.clone().unwrap().to_string();
                    // Get receipts
                    let receipts = get_receipts(&block, &clone_web3).await;
                    info!(
                        "Got ETHEREUM {} receipts of block: {}",
                        receipts.len(),
                        block_number
                    );

                    let eth_block = Block {
                        version: clone_version.clone(),
                        timestamp: block.timestamp.as_u64(),
                        block,
                        receipts,
                    };

                    let generic_data_proto =
                        _create_generic_block(block_hash, block_number, &eth_block, clone_version);
                    info!(
                        "Sending ETHEREUM as generic data: {:?}",
                        &generic_data_proto.block_number
                    );
                    chan_clone.send(generic_data_proto).unwrap();
                } else {
                    info!("Got ETHEREUM block error {:?}", &block);
                }
            });
        }
        got_block_number = Some(latest_block_number);
    }
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
