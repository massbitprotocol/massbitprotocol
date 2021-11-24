use crate::solana::model::EncodedConfirmedBlockWithSlot;
use log::{debug, info, warn};

use massbit::prelude::tokio::sync::{OwnedSemaphorePermit, Semaphore};
use massbit::prelude::tokio::time::sleep;

use massbit_common::prelude::tokio::time::{timeout, Duration};
use massbit_common::NetworkType;
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::UiTransactionEncoding;
use std::error::Error;
use std::{sync::Arc, time::Instant};
use tokio::sync::mpsc;

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const BLOCK_AVAILABLE_MARGIN: u64 = 100;
const RPC_BLOCK_ENCODING: UiTransactionEncoding = UiTransactionEncoding::Base64;
const GET_BLOCK_TIMEOUT_SEC: u64 = 60;
const BLOCK_BATCH_SIZE: u64 = 10;
const GET_NEW_SLOT_DELAY_MS: u64 = 500;

pub async fn loop_get_block(
    chan: mpsc::Sender<EncodedConfirmedBlockWithSlot>,
    start_block: &Option<u64>,
    _network: &NetworkType,
    client: &Arc<RpcClient>,
) -> Result<(), Box<dyn Error>> {
    info!("Start get block Solana from: {:?}", start_block);
    let mut last_indexed_slot: Option<u64> = start_block.map(|start_block| start_block + 1);
    //fix_one_thread_not_receive(&chan);
    let sem = Arc::new(Semaphore::new(2 * BLOCK_BATCH_SIZE as usize));
    loop {
        if chan.is_closed() {
            return Err("Stream is closed!".into());
        }
        match client.get_slot() {
            Ok(new_slot) => {
                // Root is finalized block in Solana
                let current_root = new_slot - BLOCK_AVAILABLE_MARGIN;
                //info!("Root: {:?}",new_info.root);
                match last_indexed_slot {
                    Some(value_last_indexed_slot) => {
                        if current_root == value_last_indexed_slot {
                            sleep(Duration::from_millis(GET_NEW_SLOT_DELAY_MS)).await;
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
                        let mut blocks: Vec<EncodedConfirmedBlockWithSlot> = blocks
                            .into_iter()
                            .filter_map(|res_block| res_block.ok().and_then(|res| res.ok()))
                            .collect();
                        blocks.sort_by(|a, b| a.block_slot.cmp(&b.block_slot));

                        for block in blocks.into_iter() {
                            let block_slot = block.block_slot;
                            debug!("gRPC sending block {}", &block_slot);
                            if !chan.is_closed() {
                                let start = Instant::now();
                                let send_res = chan.send(block).await;
                                if send_res.is_ok() {
                                    info!(
                                        "gRPC successfully sending block {} in {:?}",
                                        &block_slot,
                                        start.elapsed()
                                    );
                                } else {
                                    warn!("gRPC unsuccessfully sending block {}", &block_slot);
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
                eprintln!("Get slot error: {:?}", err);
                sleep(Duration::from_millis(GET_NEW_SLOT_DELAY_MS)).await;
                continue;
            }
        }
    }
    Ok(())
}

async fn get_block(
    client: Arc<RpcClient>,
    permit: OwnedSemaphorePermit,
    block_slot: u64,
) -> Result<EncodedConfirmedBlockWithSlot, Box<dyn Error + Send + Sync + 'static>> {
    let _permit = permit;
    //info!("Starting RPC get Block {}", block_slot);
    let now = Instant::now();
    let block = client.get_block_with_encoding(block_slot, RPC_BLOCK_ENCODING);
    let elapsed = now.elapsed();
    match block {
        Ok(block) => {
            info!(
                "Finished RPC get Block: {:?}, time: {:?}, hash: {}",
                block_slot, elapsed, &block.blockhash
            );
            Ok(EncodedConfirmedBlockWithSlot { block_slot, block })
        }
        _ => {
            info!(
                "Cannot get RPC get Block: {:?}, Error:{:?}, time: {:?}",
                block_slot, block, elapsed
            );
            Err(format!("Error cannot get block").into())
        }
    }
}
