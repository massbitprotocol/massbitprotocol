use crate::CONFIG;


use log::{debug, info, warn};
use massbit::firehose::bstream::{BlockResponse, ChainType};
use massbit::prelude::serde_json::{json};
use massbit::prelude::tokio::sync::{OwnedSemaphorePermit, Semaphore};
use massbit::prelude::tokio::time::sleep;
use massbit_chain_solana::data_type::{
    decode_encoded_block, decode_transaction, get_list_log_messages_from_encoded_block, Pubkey,
    SolanaBlock as Block, SolanaFilter,
};
use massbit_common::prelude::tokio::time::{timeout, Duration};
use massbit_common::NetworkType;
use solana_client::client_error::{Result as ClientResult};
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;
use solana_client::{rpc_client::RpcClient};

use solana_sdk::account::Account;
use solana_sdk::clock::Slot;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    ConfirmedBlock, EncodedConfirmedBlock, EncodedConfirmedTransaction, UiTransactionEncoding,
};
use std::error::Error;
use std::str::FromStr;
use std::{sync::Arc, time::Instant};
use tokio::sync::mpsc;
use tonic::Status;

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Solana;
const VERSION: &str = "1.7.0";
const BLOCK_AVAILABLE_MARGIN: u64 = 100;
const RPC_BLOCK_ENCODING: UiTransactionEncoding = UiTransactionEncoding::Base64;
const GET_BLOCK_TIMEOUT_SEC: u64 = 60;
const BLOCK_BATCH_SIZE: u64 = 10;
const GET_NEW_SLOT_DELAY_MS: u64 = 500;
const TRANSACTION_BATCH_SIZE: usize = 100;
// The max value is 1000
const LIMIT_FILTER_RESULT: usize = 1000;

#[derive(Debug)]
struct ResultFilterTransaction {
    txs: Vec<RpcConfirmedTransactionStatusWithSignature>,
    last_tx_signature: Option<Signature>,
    is_done: bool,
}

fn getTransactions(
    client: &Arc<RpcClient>,
    txs: &Vec<RpcConfirmedTransactionStatusWithSignature>,
    start_tx: &mut usize,
) -> ClientResult<Vec<ClientResult<EncodedConfirmedTransaction>>> {
    // Param:
    // [
    //     "5JNE26BL1FGGNdjSFXDmVrVYLv7oayUYNErSE4KqMuDTiX4rSeUC9yFtLFMwpkqFAEQNm22AvUanfd8PXAH4pukm",
    //     "json"
    //]

    //Get transaction invert direction from the txs
    let end_tx = match *start_tx >= TRANSACTION_BATCH_SIZE {
        true => *start_tx - TRANSACTION_BATCH_SIZE,
        false => 0,
    };

    let call_txs = &txs[end_tx..*start_tx];
    *start_tx = end_tx;

    println!(
        "getTransactions: {} remand transactions for getting",
        start_tx
    );
    let params = call_txs
        .iter()
        .rev()
        .map(|tx| json!([tx.signature, "base64"]))
        .collect();

    //    let res: ClientResult<Vec<EncodedConfirmedTransaction>> =
    let res: ClientResult<Vec<ClientResult<EncodedConfirmedTransaction>>> =
        client.send_batch(RpcRequest::GetTransaction, params);
    debug!("res: {:?}", res);
    res
}

fn getFilterConfirmedTransactionStatus(
    filter: &SolanaFilter,
    client: &Arc<RpcClient>,
    before_tx_signature: &Option<Signature>,
    first_slot: &Option<Slot>,
) -> ResultFilterTransaction {
    let mut txs: Vec<RpcConfirmedTransactionStatusWithSignature> = vec![];
    let _is_done = false;
    for address in &filter.keys {
        let config = GetConfirmedSignaturesForAddress2Config {
            before: before_tx_signature.clone(),
            until: None,
            limit: Some(LIMIT_FILTER_RESULT),
            commitment: None,
        };
        let res = client.get_signatures_for_address_with_config(address, config);
        txs.append(&mut res.unwrap_or(vec![]));
    }

    // Fixme: Cover the case that multi addresses are in filter, now the logic is correct for filter 1 address only
    let last_tx_signature = txs
        .last()
        .map(|tx| Signature::from_str(&tx.signature).unwrap());

    // last_tx_signature.is_none: when we cannot found any result
    // txs.last().unwrap().slot < first_slot.unwrap(): when searching is out of range
    let is_done = last_tx_signature.is_none()
        || (!first_slot.is_none() && txs.last().unwrap().slot < first_slot.unwrap());

    let txs: Vec<RpcConfirmedTransactionStatusWithSignature> = txs
        .into_iter()
        .filter(|tx| {
            // Block is out of range or error
            if !tx.err.is_none() {
                debug!("Confirmed Transaction Error: {:?}", tx.err)
            }
            (first_slot.is_some() && tx.slot < first_slot.unwrap()) || tx.err.is_none()
        })
        .collect();

    ResultFilterTransaction {
        txs,
        last_tx_signature,
        is_done,
    }
}

async fn grpc_send_block(
    block: BlockResponse,
    chan: &mpsc::Sender<Result<BlockResponse, Status>>,
) -> Result<(), Box<dyn Error>> {
    let block_number = block.block_number;
    debug!("gRPC sending block {}", &block_number);
    if !chan.is_closed() {
        let send_res = chan.send(Ok(block)).await;
        if send_res.is_ok() {
            info!("gRPC successfully sending block {}", &block_number);
        } else {
            warn!("gRPC unsuccessfully sending block {}", &block_number);
        }
    } else {
        return Err("Stream is closed!".into());
    }
    Ok(())
}

pub async fn loop_get_block(
    chan: mpsc::Sender<Result<BlockResponse, Status>>,
    start_block: &Option<u64>,
    network: &NetworkType,
    client: &Arc<RpcClient>,
    filter: &SolanaFilter,
) -> Result<(), Box<dyn Error>> {
    info!(
        "Start get block Solana from: {:?} with filter {:?}",
        start_block, filter
    );
    let _config = CONFIG.get_chain_config(&CHAIN_TYPE, &network).unwrap();
    // let websocket_url = config.ws.clone();
    // let (mut _subscription_client, receiver) =
    //     PubsubClient::slot_subscribe(&websocket_url).unwrap();

    //fix_one_thread_not_receive(&chan);
    let sem = Arc::new(Semaphore::new(2 * BLOCK_BATCH_SIZE as usize));

    let mut before_tx_signature = None;

    let mut filter_txs = vec![];
    // Backward run
    info!(
        "Start get transaction backward with filter address: {:?}",
        &filter
    );
    if start_block.is_some() {
        loop {
            let now = Instant::now();
            let mut res = getFilterConfirmedTransactionStatus(
                &filter,
                client,
                &before_tx_signature,
                start_block,
            );
            debug!("res: {:?}", res);
            before_tx_signature = res.last_tx_signature;
            filter_txs.append(res.txs.as_mut());

            info!("Time to get filter transactions: {:?}. Got {:?} filtered addresses, last address: {:?}",now.elapsed(), filter_txs.len(),
        filter_txs.last());
            // No record in txs
            if res.is_done {
                break;
            }
        }
    }
    info!("Start get {} transaction forward.", filter_txs.len());
    // Forward run
    let mut start_tx: usize = filter_txs.len();
    while start_tx > 0 {
        let transactions = getTransactions(client, &filter_txs, &mut start_tx);
        match transactions {
            Ok(transactions) => {
                for transaction in transactions {
                    match transaction {
                        Ok(transaction) => {
                            let decode_transactions =
                                match decode_transaction(&transaction.transaction) {
                                    Some(transaction) => vec![transaction],
                                    None => {
                                        warn!(
                                            "transaction in block {:#?} cannot decode!",
                                            &transaction.slot
                                        );
                                        vec![]
                                    }
                                };
                            let filtered_confirmed_block = ConfirmedBlock {
                                previous_blockhash: "".to_string(),
                                blockhash: "".to_string(),
                                parent_slot: transaction.slot - 1,
                                transactions: decode_transactions,
                                rewards: vec![],
                                block_time: transaction.block_time,
                                block_height: None,
                            };
                            let generic_block = _to_generic_block(filtered_confirmed_block);
                            grpc_send_block(generic_block, &chan).await?
                        }
                        Err(e) => info!("Transaction error: {:?}", e),
                    }
                }
            }
            Err(e) => {
                warn!("Call batch transaction error: {:?}", e);
            }
        }
    }

    // start from the last indexed block
    let mut last_indexed_slot: Option<u64> = filter_txs.last().map(|tx| tx.slot);

    info!(
        "Start getting forward at the block {:?} .",
        &last_indexed_slot
    );
    // from current block run
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
                        let mut blocks: Vec<BlockResponse> = blocks
                            .into_iter()
                            .filter_map(|res_block| {
                                if let Ok(Ok(block)) = res_block {
                                    info!(
                                        "Got SOLANA block at slot : {:?}",
                                        &block.parent_slot + 1
                                    );
                                    let decode_block = decode_encoded_block(block);
                                    let filtered_block = filter.filter_block(decode_block);
                                    if filtered_block.transactions.is_empty() {
                                        println!(
                                            "Block slot {} has no match Transaction",
                                            filtered_block.parent_slot + 1
                                        );
                                        None
                                    } else {
                                        Some(_to_generic_block(filtered_block))
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect();
                        blocks.sort_by(|a, b| a.block_number.cmp(&b.block_number));
                        info!("Finished get blocks");
                        for block in blocks.into_iter() {
                            grpc_send_block(block, &chan).await?
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

fn _to_generic_block(block: solana_transaction_status::ConfirmedBlock) -> BlockResponse {
    let timestamp = (&block).block_time.unwrap();
    let list_log_messages = get_list_log_messages_from_encoded_block(&block);
    let ext_block = Block {
        version: VERSION.to_string(),
        block,
        timestamp,
        list_log_messages,
    };
    let generic_data_proto = _create_generic_block(
        ext_block.block.blockhash.clone(),
        &ext_block.block.parent_slot + 1,
        &ext_block,
    );
    generic_data_proto
}

async fn get_block(
    client: Arc<RpcClient>,
    permit: OwnedSemaphorePermit,
    block_height: u64,
) -> Result<EncodedConfirmedBlock, Box<dyn Error + Send + Sync + 'static>> {
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
            Ok(block)
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

// Helper function for direct call
fn get_rpc_client(network: NetworkType) -> Arc<RpcClient> {
    let config = CONFIG.get_chain_config(&CHAIN_TYPE, &network).unwrap();
    let json_rpc_url = config.url.clone();
    info!("Init Solana client, url: {}", json_rpc_url);
    Arc::new(RpcClient::new(json_rpc_url.clone()))
}

fn get_account_info(client: Arc<RpcClient>, pubkey: &Pubkey) -> ClientResult<Account> {
    client.get_account(pubkey)
}
