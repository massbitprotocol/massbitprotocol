use crate::CONFIG;

use log::{debug, info, warn};
use massbit::firehose::bstream::{BlockResponse, ChainType};
use massbit::prelude::serde_json::json;
use massbit::prelude::tokio::sync::{OwnedSemaphorePermit, Semaphore};
use massbit::prelude::tokio::time::sleep;
use massbit_chain_solana::data_type::{
    decode_encoded_block, decode_transaction, get_list_log_messages_from_encoded_block, Pubkey,
    SolanaBlock as Block, SolanaFilter,
};
use massbit_common::prelude::tokio::time::{timeout, Duration};
use massbit_common::NetworkType;
use solana_client::client_error::Result as ClientResult;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;

use massbit::prelude::lazy_static;
// use massbit::slog::MutexDrainError::Mutex;
use massbit_common::prelude::diesel::serialize::IsNull::No;
//use solana_runtime::contains::Contains;
use massbit_common::prelude::diesel::RunQueryDsl;
use solana_sdk::account::Account;
use solana_sdk::clock::Slot;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    ConfirmedBlock, EncodedConfirmedBlock, EncodedConfirmedTransaction, TransactionWithStatusMeta,
    UiTransactionEncoding,
};
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc;
use tonic::Status;

// Check https://github.com/tokio-rs/prost for enum converting in rust protobuf
const CHAIN_TYPE: ChainType = ChainType::Solana;
const VERSION: &str = "1.7.0";
const BLOCK_AVAILABLE_MARGIN: u64 = 100;
const RPC_BLOCK_ENCODING: UiTransactionEncoding = UiTransactionEncoding::Base64;
const GET_BLOCK_TIMEOUT_SEC: u64 = 120;
const BLOCK_BATCH_SIZE: u64 = 2;
//massbit 2: 1-> 5,695ms, 10->7.54s, 50 -> 14.2ms, 100 -> 17.8ms
//massbit 3: 20-> ,50 -> 8.983ms, 100 -> 10.289ms
const GET_NEW_SLOT_DELAY_MS: u64 = 500;
const TRANSACTION_BATCH_SIZE: usize = 100;
// The max value is 1000
const LIMIT_FILTER_RESULT: usize = 1000;

lazy_static! {
    pub static ref BLOCK_COUNT: AtomicUsize = AtomicUsize::new(0);
    pub static ref GET_BLOCK_DURATION_MS: AtomicUsize = AtomicUsize::new(0);
    pub static ref BLOCK_TIMEOUT_COUNT: AtomicUsize = AtomicUsize::new(0);
}

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
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if !chan.is_closed() {
        chan.send(Ok(block)).await;
    } else {
        return Err("Stream is closed!".into());
    }
    Ok(())
}

fn decode_and_filter(
    filter: Arc<SolanaFilter>,
    block: EncodedConfirmedBlock,
) -> Option<solana_transaction_status::ConfirmedBlock> {
    let decode_block = decode_encoded_block(block);
    let filtered_block = filter.filter_block(decode_block);
    if filtered_block.transactions.is_empty() {
        println!(
            "Block slot {} has no match Transaction",
            &filtered_block.parent_slot + 1
        );
        None
    } else {
        Some(filtered_block)
    }
}
#[derive(Clone, Debug, Default)]
struct SendQueue {
    queue: HashMap<Slot, Option<ConfirmedBlock>>,
    sending_slot: u64,
}

async fn insert_and_prepare_send(
    send_queue: Arc<Mutex<SendQueue>>,
    block: Option<ConfirmedBlock>,
    block_slot: &u64,
) -> Option<BlockResponse> {
    let mut send_queue = send_queue.lock().unwrap();
    let mut count_send_block = 0;
    // Insert block into queue
    send_queue.queue.insert(*block_slot, block);
    // Check if there are blocks that could be sending
    let mut end_slot: Option<u64> = None;
    let mut check_slot = send_queue.sending_slot;
    let mut blocks = vec![];
    loop {
        if send_queue.queue.contains_key(&check_slot) {
            let block = send_queue.queue.remove(&check_slot);
            if let Some(Some(block)) = block {
                blocks.push((block, check_slot));
            }
            check_slot = check_slot + 1;
        } else {
            send_queue.sending_slot = check_slot;
            break;
        }
    }

    drop(send_queue);
    if count_send_block != 0 {
        let block_response = _to_generic_block(blocks);
        info!("Packed {} blocks to send", count_send_block);
        Some(block_response)
    } else {
        None
    }
}

pub async fn loop_get_block(
    chan: mpsc::Sender<Result<BlockResponse, Status>>,
    start_block: &Option<u64>,
    network: &NetworkType,
    client: &Arc<RpcClient>,
    filter: &SolanaFilter,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!(
        "Start get block Solana from: {:?} with filter {:?}",
        start_block, filter
    );
    let filter = Arc::new(filter.clone());

    let _config = CONFIG.get_chain_config(&CHAIN_TYPE, &network).unwrap();
    // let websocket_url = config.ws.clone();
    // let (mut _subscription_client, receiver) =
    //     PubsubClient::slot_subscribe(&websocket_url).unwrap();

    //fix_one_thread_not_receive(&chan);
    let sem = Arc::new(Semaphore::new(2 * BLOCK_BATCH_SIZE as usize));

    let mut before_tx_signature = None;

    let mut filter_txs = vec![];

    //******************* Backward check ***************************//
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
    //******************* Forward run ***************************//
    info!("Start get {} transaction forward.", filter_txs.len());

    let mut start_tx: usize = filter_txs.len();
    while start_tx > 0 {
        let transactions = getTransactions(client, &filter_txs, &mut start_tx);
        // Check transactions
        match transactions {
            Ok(transactions) => {
                // Decode and group transactions into the same block groups
                let mut group_transactions: HashMap<Slot, Vec<TransactionWithStatusMeta>> =
                    HashMap::new();
                for transaction in transactions {
                    match transaction {
                        Ok(transaction) => {
                            // Decode the transaction
                            match decode_transaction(&transaction.transaction) {
                                Some(decoded_transaction) => {
                                    group_transactions
                                        .entry(transaction.slot)
                                        .or_insert(vec![])
                                        .push(decoded_transaction);
                                }
                                None => {
                                    warn!(
                                        "transaction in block {:#?} cannot decode!",
                                        &transaction.slot
                                    );
                                    continue;
                                }
                            };
                        }
                        Err(e) => continue,
                    }
                }

                let filtered_confirmed_blocks_with_number: Vec<(ConfirmedBlock, u64)> =
                    group_transactions
                        .into_iter()
                        .map(|(block_number, transactions)| {
                            let filtered_confirmed_block = ConfirmedBlock {
                                previous_blockhash: Default::default(),
                                blockhash: Default::default(),
                                parent_slot: Default::default(),
                                transactions,
                                rewards: Default::default(),
                                block_time: Default::default(),
                                block_height: Default::default(),
                            };
                            (filtered_confirmed_block, block_number)
                        })
                        .collect();
                if !filtered_confirmed_blocks_with_number.is_empty() {
                    info!(
                        "There are {} filtered Block in array.",
                        filtered_confirmed_blocks_with_number.len()
                    );
                    let generic_block = _to_generic_block(filtered_confirmed_blocks_with_number);
                    grpc_send_block(generic_block, &chan).await?
                }
            }
            Err(e) => {
                warn!("Call batch transaction error: {:?}", e);
            }
        }
    }

    // start from the last indexed block
    let mut last_indexed_slot: u64 = match filter_txs.last().map(|tx| tx.slot) {
        Some(last_indexed_slot) => last_indexed_slot,
        None => {
            // let get_slot = || {
            //     client.get_slot().unwrap_or_else(get_slot)
            // };
            // get_slot()
            let mut last_indexed_slot = 0;
            loop {
                match client.get_slot() {
                    Ok(_last_indexed_slot) => {
                        last_indexed_slot = _last_indexed_slot - BLOCK_AVAILABLE_MARGIN;
                        break;
                    }
                    Err(_) => continue,
                }
            }
            last_indexed_slot
        }
    };

    info!(
        "Start getting forward at the block {:?} .",
        &last_indexed_slot
    );
    //******************* From current block run ***************************//
    // Last sending block number
    let send_queue: Arc<Mutex<SendQueue>> = Arc::new(Mutex::new(SendQueue {
        queue: HashMap::new(),
        sending_slot: last_indexed_slot,
    }));
    loop {
        if chan.is_closed() {
            return Err("Stream is closed!".into());
        }
        match client.get_slot() {
            Ok(new_slot) => {
                // Root is finalized block in Solana
                let current_root = new_slot - BLOCK_AVAILABLE_MARGIN;
                //info!("Root: {:?}",new_info.root);
                if current_root == last_indexed_slot {
                    sleep(Duration::from_millis(GET_NEW_SLOT_DELAY_MS)).await;
                    continue;
                }
                info!(
                    "Latest stable block: {}, Pending block: {}",
                    current_root,
                    current_root - last_indexed_slot
                );
                // let mut tasks = vec![];
                let number_get_slot = (current_root - last_indexed_slot).min(BLOCK_BATCH_SIZE);
                let block_range = last_indexed_slot..(last_indexed_slot + number_get_slot);

                for block_slot in block_range {
                    let new_client = client.clone();
                    let chan_clone = chan.clone();
                    let filter_clone = filter.clone();
                    let permit = Arc::clone(&sem).acquire_owned().await.unwrap();
                    let mut send_queue_clone = send_queue.clone();
                    tokio::spawn(async move {
                        let res = timeout(
                            Duration::from_secs(GET_BLOCK_TIMEOUT_SEC),
                            get_block(new_client, permit, block_slot),
                        )
                        .await;
                        info!(
                            "Finish tokio::spawn for getting block number: {:?}",
                            &block_slot
                        );

                        match res {
                            Ok(Ok((block, slot))) => {
                                let decoded_block = decode_and_filter(filter_clone, block);
                                let block_response = insert_and_prepare_send(
                                    send_queue_clone,
                                    decoded_block,
                                    &block_slot,
                                )
                                .await;
                                // if let Some(block_response) = block_response {
                                //     grpc_send_block(block_response, &chan_clone).await;
                                //     info!("Send block_response to indexer-manager");
                                // }
                            }
                            Err(_) | Ok(Err(_)) => {
                                warn!("get_block timed out at block number {}", &block_slot);
                                BLOCK_TIMEOUT_COUNT.fetch_add(1, Ordering::SeqCst);
                                let block_response =
                                    insert_and_prepare_send(send_queue_clone, None, &block_slot)
                                        .await;
                                if let Some(block_response) = block_response {
                                    grpc_send_block(block_response, &chan_clone).await;
                                    info!("Send block_response to indexer-manager");
                                }
                            }
                        }
                    });
                }

                last_indexed_slot = last_indexed_slot + number_get_slot;
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

fn _create_generic_block(blocks: &Vec<Block>) -> BlockResponse {
    let generic_data = BlockResponse {
        version: VERSION.to_string(),
        payload: serde_json::to_vec(blocks).unwrap(),
    };
    generic_data
}

fn _to_generic_block(blocks_with_number: Vec<(ConfirmedBlock, u64)>) -> BlockResponse {
    let ext_blocks: Vec<Block> = blocks_with_number
        .into_iter()
        .map(|(block, block_number)| {
            let timestamp = (&block).block_time.unwrap_or_default();
            let list_log_messages = get_list_log_messages_from_encoded_block(&block);
            Block {
                version: VERSION.to_string(),
                block,
                block_number,
                timestamp,
                list_log_messages,
            }
        })
        .collect();

    let generic_data_proto = _create_generic_block(&ext_blocks);
    generic_data_proto
}

async fn get_block(
    client: Arc<RpcClient>,
    permit: OwnedSemaphorePermit,
    block_number: u64,
) -> Result<(EncodedConfirmedBlock, u64), Box<dyn Error + Send + Sync + 'static>> {
    let _permit = permit;
    info!("Starting RPC get Block {}", block_number);
    let now = Instant::now();
    let block = client.get_block_with_encoding(block_number, RPC_BLOCK_ENCODING);
    let elapsed = now.elapsed();
    let duration = GET_BLOCK_DURATION_MS.fetch_add(elapsed.as_millis() as usize, Ordering::SeqCst)
        + elapsed.as_millis() as usize;
    let block_count = BLOCK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    match block {
        Ok(block) => {
            info!(
                "Finished RPC get Block: {:?}, get-block duration: {:?}, average get-block duration: {} ms, timeout ratio: {}, hash: {}",
                block_number, elapsed,(duration/block_count), (BLOCK_TIMEOUT_COUNT.load(Ordering::SeqCst) as f32 / block_number as f32), &block.blockhash
            );
            Ok((block, block_number))
        }
        Err(e) => {
            if format!("{:?}", &e).contains("TimedOut") {
                BLOCK_TIMEOUT_COUNT.fetch_add(1, Ordering::SeqCst);
            }
            info!(
                "Cannot get RPC get Block: {:?}, Error:{:?}, get-block duration: {:?}, average get-block duration: {} ms, timeout ratio: {}",
                block_number, &e, elapsed, (duration/block_count),(BLOCK_TIMEOUT_COUNT.load(Ordering::SeqCst) as f32 / block_number as f32)
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
