use crate::CONFIG;
use log::{debug, info, warn};
use massbit::firehose::bstream::{BlockResponse, ChainType};
use massbit::prelude::tokio::sync::{OwnedSemaphorePermit, Semaphore};
use massbit_chain_solana::data_type::{
    get_list_log_messages_from_encoded_block, SolanaEncodedBlock as Block,
};
use massbit_common::prelude::tokio::time::{sleep, timeout, Duration};
use massbit_common::NetworkType;
use solana_client::rpc_response::SlotInfo;
use solana_client::{pubsub_client::PubsubClient, rpc_client::RpcClient};
use solana_transaction_status::UiTransactionEncoding;
use std::error::Error;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use tokio::sync::mpsc;
use tonic::Status;

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Solana;
const VERSION: &str = "1.6.16";
const BLOCK_AVAILABLE_MARGIN: u64 = 100;
const RPC_BLOCK_ENCODING: UiTransactionEncoding = UiTransactionEncoding::Base64;
const GET_BLOCK_TIMEOUT_SEC: u64 = 60;
const BLOCK_BATCH_SIZE: u64 = 10;

pub async fn loop_get_block(
    chan: mpsc::Sender<Result<BlockResponse, Status>>,
    start_block: &Option<u64>,
    network: &NetworkType,
    client: &Arc<RpcClient>,
) -> Result<(), Box<dyn Error>> {
    info!("Start get block Solana from: {:?}", start_block);
    let config = CONFIG.get_chain_config(&CHAIN_TYPE, &network).unwrap();
    let websocket_url = config.ws.clone();
    let (mut subscription_client, receiver) = PubsubClient::slot_subscribe(&websocket_url).unwrap();
    let mut last_indexed_slot: Option<u64> = start_block.map(|start_block| start_block + 1);
    //fix_one_thread_not_receive(&chan);
    let sem = Arc::new(Semaphore::new(2 * BLOCK_BATCH_SIZE as usize));
    loop {
        match receiver.recv() {
            Ok(new_info) => {
                // Root is finalized block in Solana
                let current_root = new_info.root - BLOCK_AVAILABLE_MARGIN;
                //info!("Root: {:?}",new_info.root);
                match last_indexed_slot {
                    Some(value_last_indexed_slot) => {
                        if current_root == value_last_indexed_slot {
                            continue;
                        }
                        info!(
                            "Latest stable block: {}, Pending block: {}",
                            current_root,
                            current_root - value_last_indexed_slot
                        );
                        let mut tasks = vec![];
                        let number_get_slot =
                            (current_root - value_last_indexed_slot).min(BLOCK_BATCH_SIZE);
                        let block_range =
                            value_last_indexed_slot..(value_last_indexed_slot + number_get_slot);
                        for block_slot in block_range {
                            let new_client = client.clone();
                            let permit = Arc::clone(&sem).acquire_owned().await.unwrap();
                            tasks.push(tokio::spawn(async move {
                                let res = timeout(
                                    Duration::from_secs(GET_BLOCK_TIMEOUT_SEC),
                                    get_block(new_client, permit, block_slot),
                                )
                                .await;
                                if res.is_err() {
                                    warn!("get_block timed out at block height {}", &block_slot);
                                };
                                info!(
                                    "Finish tokio::spawn for getting block height: {:?}",
                                    &block_slot
                                );
                                res.unwrap()
                            }));
                        }
                        let blocks: Vec<Result<_, _>> = futures03::future::join_all(tasks).await;
                        let mut blocks: Vec<BlockResponse> = blocks
                            .into_iter()
                            .filter_map(|res_block| {
                                if let Ok(Ok(block)) = res_block {
                                    info!("Got SOLANA block : {:?}", &block.block_number);
                                    Some(block)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        blocks.sort_by(|a, b| a.block_number.cmp(&b.block_number));
                        info!("Finished get blocks");
                        for block in blocks.into_iter() {
                            let block_number = block.block_number;
                            debug!("gRPC sending block {}", &block_number);
                            if !chan.is_closed() {
                                let send_res = chan.send(Ok(block as BlockResponse)).await;
                                if send_res.is_ok() {
                                    info!("gRPC successfully sending block {}", &block_number);
                                } else {
                                    warn!("gRPC unsuccessfully sending block {}", &block_number);
                                }
                            } else {
                                return Err("Stream is closed!".into());
                            }
                        }
                        last_indexed_slot = last_indexed_slot
                            .map(|last_indexed_slot| last_indexed_slot + number_get_slot);
                    }
                    _ => last_indexed_slot = Some(current_root),
                };
            }
            Err(err) => {
                eprintln!("disconnected: {}", err);
                break;
            }
        }
    }
    Ok(())
}

fn _create_generic_block(block_hash: String, block_number: u64, block: &Block) -> BlockResponse {
    let generic_data = BlockResponse {
        chain_type: CHAIN_TYPE as i32,
        version: VERSION.to_string(),
        block_hash,
        block_number,
        payload: serde_json::to_vec(block).unwrap(),
    };
    generic_data
}

async fn get_block(
    client: Arc<RpcClient>,
    permit: OwnedSemaphorePermit,
    block_height: u64,
) -> Result<BlockResponse, Box<dyn Error + Send + Sync + 'static>> {
    let _permit = permit;
    info!("Starting RPC get Block {}", block_height);
    let now = Instant::now();
    let block = client.get_block_with_encoding(block_height, RPC_BLOCK_ENCODING);
    let elapsed = now.elapsed();
    match block {
        Ok(block) => {
            info!(
                "Finished RPC get Block: {:?}, time: {:?}, hash: {}",
                block_height, elapsed, &block.blockhash
            );
            let timestamp = (&block).block_time.unwrap();
            let list_log_messages = get_list_log_messages_from_encoded_block(&block);
            let ext_block = Block {
                version: VERSION.to_string(),
                block,
                timestamp,
                list_log_messages,
            };
            let generic_data_proto =
                _create_generic_block(ext_block.block.blockhash.clone(), block_height, &ext_block);
            Ok(generic_data_proto)
        }
        _ => {
            info!(
                "Cannot get RPC get Block: {:?}, Error:{:?}, time: {:?}",
                block_height, block, elapsed
            );
            Err(format!("Error cannot get block").into())
        }
    }
}
