use crate::{
    grpc_stream::stream_mod::{ChainType, DataType, GenericDataProto},
    CONFIG,
};
use log::{info, warn};
use massbit_chain_ethereum::data_type::{EthereumBlock as Block, LightEthereumBlock};
use std::hash::Hash;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use web3;
use web3::api::SubscriptionStream;
use web3::transports::{Http, WebSocket};
use web3::types::BlockHeader;
use web3::{
    futures::{future, StreamExt},
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

enum Web3Connection {
    http(Web3<Http>),
    ws(Web3<WebSocket>),
}

fn fix_one_thread_not_receive(chan: &broadcast::Sender<GenericDataProto>) {
    // Todo: More clean solution for broadcast channel
    let mut rx = chan.subscribe();
    tokio::spawn(async move {
        loop {
            let _ = rx.recv().await;
        }
    });
}

async fn fetch_receipt_from_ethereum_client(
    web3_http: &Web3<Http>,
    transaction_hash: &H256,
) -> Result<TransactionReceipt, Box<dyn std::error::Error + Send + Sync + 'static>> {
    match web3_http.eth().transaction_receipt(*transaction_hash).await {
        Ok(Some(receipt)) => Ok(receipt),
        Ok(None) => Err("Could not find transaction receipt".into()),
        Err(error) => Err(format!("Failed to fetch transaction receipt: {:?}", error).into()),
    }
}

async fn wait_for_new_block_http(web3_http: &Web3<Http>, got_block_number: Option<u64>) -> u64 {
    loop {
        let block_header = web3_http.eth().block(Web3BlockNumber::Latest.into()).await;
        if let Ok(Some(block_header)) = block_header {
            let latest_block_number = block_header.number.unwrap().as_u64();
            if let None = got_block_number {
                return latest_block_number;
            } else if (latest_block_number > got_block_number.unwrap()) {
                return latest_block_number;
            }
        }
        sleep(Duration::from_millis(PULLING_INTERVAL)).await;
    }
}

async fn wait_for_new_block_ws(
    sub: &mut SubscriptionStream<WebSocket, BlockHeader>,
    got_block_number: Option<u64>,
) -> u64 {
    let mut latest_block_number = 0;
    // Wait for new block
    sub.take(1)
        .for_each(|x| {
            println!("Got: {:?}", x);
            latest_block_number = x.unwrap().number.unwrap().as_u64();
            future::ready(())
        })
        .await;
    latest_block_number
}

async fn get_receipts(
    web3_http: &Web3<Http>,
    transactions: &Vec<Transaction>,
) -> Vec<TransactionReceipt> {
    let mut receipts = Vec::new();
    for transaction in transactions {
        let res_receipt = fetch_receipt_from_ethereum_client(web3_http, &transaction.hash).await;
        if let Ok(receipt) = res_receipt {
            receipts.push(receipt);
        }
    }

    receipts
}

pub async fn loop_get_block(chan: broadcast::Sender<GenericDataProto>) {
    info!("Start get block {:?}", CHAIN_TYPE);
    info!("Init Ethereum adapter");
    let exit = Arc::new(AtomicBool::new(false));
    let config = CONFIG.chains.get(&CHAIN_TYPE).unwrap();
    let websocket_url = config.ws.clone();
    let transport = web3::transports::WebSocket::new(websocket_url.as_str())
        .await
        .expect("Cannot connect websocket url for {:?}!");

    let mut web3_ws = web3::Web3::new(transport);
    let http_url = config.url.clone();
    let transport =
        web3::transports::Http::new(http_url.as_str()).expect("Cannot connect http url for {:?}!");

    let web3_http = web3::Web3::new(transport);

    let mut sub = web3_ws.eth_subscribe().subscribe_new_heads().await.unwrap();
    let mut version;
    if USE_WEBSOCKET {
        println!("Got subscription id: {:?}", sub.id());
        // Get version
        version = web3_ws
            .net()
            .version()
            .await
            .unwrap_or("Cannot get version".to_string());
    } else {
        // Get version
        version = web3_http
            .net()
            .version()
            .await
            .unwrap_or("Cannot get version".to_string());
    }

    // let mut sub = web3.eth_subscribe().subscribe_new_heads().await.unwrap();
    // println!("Got subscription id: {:?}", sub.id());

    fix_one_thread_not_receive(&chan);
    let mut got_block_number = None;
    loop {
        if exit.load(Ordering::Relaxed) {
            eprintln!("{}", "exit".to_string());
            break;
        }
        let mut latest_block_number;
        if USE_WEBSOCKET {
            latest_block_number = wait_for_new_block_ws(&mut sub, got_block_number).await;
        } else {
            latest_block_number = wait_for_new_block_http(&web3_http, got_block_number).await;
        }

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
            let mut clone_web3_http = web3_http.clone();
            let mut clone_web3_ws = web3_ws.clone();

            tokio::spawn(async move {
                // Get block
                info!("Getting ETHEREUM block {}", block_number);
                let mut block;
                if USE_WEBSOCKET {
                    block = clone_web3_ws
                        .eth()
                        .block_with_txs(BlockId::Number(Web3BlockNumber::from(block_number)))
                        .await;
                } else {
                    block = clone_web3_http
                        .eth()
                        .block_with_txs(BlockId::Number(Web3BlockNumber::from(block_number)))
                        .await;
                }

                if let Ok(Some(block)) = block {
                    //println!("Got ETHEREUM Block {:?}",block);
                    // Convert to generic
                    let block_hash = block.hash.clone().unwrap().to_string();

                    let eth_block = Block {
                        version: clone_version.clone(),
                        timestamp: block.timestamp.as_u64(),
                        block,
                        // Todo: Add receipts. Now hardcode empty.
                        receipts: vec![],
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
