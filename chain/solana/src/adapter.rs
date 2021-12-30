use super::types::ChainConfig;
use crate::chain::Chain;
use crate::data_source::DataSource;
use crate::storage::BlockStorage;
use crate::types::{BlockInfo, ConfirmedBlockWithSlot, Pubkey};
use crate::{LIMIT_FILTER_RESULT, SOLANA_NETWORKS, TRANSACTION_BATCH_SIZE};
use log::{debug, error, info, log, warn};
use massbit::blockchain as bc;
use massbit::blockchain::HostFn;
use massbit::prelude::*;
use serde_json::json;
use solana_client::client_error::Result as ClientResult;
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_client::rpc_request::RpcRequest;
use solana_program::clock::Slot;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiInstruction::{Compiled, Parsed};
use solana_transaction_status::{
    ConfirmedBlock, ConfirmedTransaction, EncodedConfirmedBlock, EncodedConfirmedTransaction,
    InnerInstructions, TransactionStatusMeta, TransactionTokenBalance, TransactionWithStatusMeta,
    UiInnerInstructions, UiTransactionEncoding, UiTransactionTokenBalance,
};
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use std::thread::Thread;
use std::time::Instant;
use tokio::sync::mpsc::Sender;
use tokio::sync::{AcquireError, Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::time::error::Elapsed;
use tokio::time::{sleep, timeout};

const BLOCK_AVAILABLE_MARGIN: u64 = 100;
const GET_NEW_SLOT_DELAY_MS: u64 = 500;
const BLOCK_BATCH_SIZE: usize = 10;
const GET_BLOCK_TIMEOUT_SEC: u64 = 60;
const RPC_BLOCK_ENCODING: UiTransactionEncoding = UiTransactionEncoding::Base64;
const QUEUE_GET_BLOCK_TIME: usize = 40 - 1;
const WINDOW_TIME: u128 = 10000;
#[derive(Clone)]
pub struct SolanaAdapter {
    pub rpc_client: Arc<RpcClient>,
    storage: Option<Arc<Box<dyn BlockStorage + Sync + Send>>>,
    //Time in ms when client send request to server
    request_times: Arc<Mutex<VecDeque<Instant>>>,
    network: String,
    sem: Arc<Semaphore>,
}

impl SolanaAdapter {
    pub fn new(
        storage: Option<Arc<Box<dyn BlockStorage + Sync + Send>>>,
        config: &ChainConfig,
    ) -> Self {
        info!("Init Solana client with url: {:?}", &config.url);
        let rpc_client = Arc::new(RpcClient::new(config.url.clone()));
        info!("Finished init Solana client");
        SolanaAdapter {
            rpc_client,
            storage,
            request_times: Arc::new(Mutex::new(VecDeque::with_capacity(QUEUE_GET_BLOCK_TIME))),
            network: config.name.clone(),
            sem: Arc::new(Semaphore::new(2 * BLOCK_BATCH_SIZE)),
        }
    }
    pub async fn acquire_owned(&self) -> Result<OwnedSemaphorePermit, AcquireError> {
        log::info!(
            "Available semaphore limit on network {:?} is {:?}",
            &self.network,
            self.sem.available_permits()
        );
        Arc::clone(&self.sem).acquire_owned().await
    }
    pub fn get_available_permits(&self) -> usize {
        self.sem.available_permits()
    }
    pub fn get_latest_block(&self) -> ClientResult<Slot> {
        self.rpc_client.clone().get_slot()
    }
    pub fn get_block_slots(&self, slot: Slot) -> ClientResult<Vec<Slot>> {
        self.rpc_client.clone().get_blocks(slot, None)
    }
    pub async fn get_block_data(
        &self,
        block_slot: Slot,
    ) -> Result<ConfirmedBlockWithSlot, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let now = Instant::now();
        //Check if need sleep for awhile to avoid rate limit 40 request per 10 seconds
        self.avoid_request_limit().await;
        let block = self
            .rpc_client
            .get_block_with_encoding(block_slot, RPC_BLOCK_ENCODING);
        let elapsed = now.elapsed();
        match block {
            Ok(block) => {
                info!(
                    "Finished get Block: {:?} from network {:?}, time: {:?}, hash: {}",
                    block_slot, &self.network, elapsed, &block.blockhash
                );
                //Store block data in cache
                Ok(ConfirmedBlockWithSlot {
                    block_slot,
                    block: Some(Self::decode_encoded_block(block)),
                })
            }
            Err(ref err) => {
                info!(
                    "Cannot get RPC get Block: {:?}, Error:{:?}, time: {:?}",
                    block_slot, err, elapsed
                );
                //Todo: implement retry get missing block
                //Send None to broadcaster -
                Err(format!("Error cannot get block").into())
            }
        }
    }
    async fn avoid_request_limit(&self) {
        let request_times = self.request_times.clone();
        let mut queue = request_times.lock().await;
        //Remove old request times (elapsed > 10secs)
        let mut counter = 0;
        let mut flag = queue.len() >= QUEUE_GET_BLOCK_TIME;
        while flag {
            if let Some(first) = queue.pop_front() {
                counter += 1;
                let elapsed_time = first.elapsed().as_millis();
                if elapsed_time < 10000 {
                    //If queue is full then sleep for a while
                    if queue.len() >= QUEUE_GET_BLOCK_TIME - 1 {
                        log::info!("Fist block in WINDOW_TIME elapses in {:?}, sleep for {:?} ms before send new request.", first.elapsed(), 10000 - first.elapsed().as_millis());
                        sleep(Duration::from_millis(10000u64 - elapsed_time as u64)).await;
                    }
                    flag = false;
                }
                // continue remove old elements
            } else {
                //Queue is empty
                flag = false;
            }
        }
        if counter > 0 {
            log::info!("Remove {:?} elements from request_times queue for network {:?}. Remained elements in queue {:?}. Oldest request sent at {:?}", 
                counter, &self.network, queue.len(), queue.get(0).and_then(|time| Some(time.elapsed())));
        }
        queue.push_back(Instant::now());
    }
    pub fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        first_slot: Option<u64>,
        end_signature: Option<Signature>,
    ) -> Vec<String> {
        let mut before_signature = end_signature;
        let mut res_signatures = vec![];
        info!(
            "Get history blocks for address {:?} from block {:?} to signature {:?}",
            address, first_slot, &end_signature
        );
        let first_slot = first_slot.unwrap_or_default();
        loop {
            let now = Instant::now();
            let config = GetConfirmedSignaturesForAddress2Config {
                before: before_signature,
                until: None,
                limit: Some(LIMIT_FILTER_RESULT),
                commitment: None,
            };
            match self
                .rpc_client
                .get_signatures_for_address_with_config(address, config)
            {
                Ok(txs) => {
                    if txs.is_empty() {
                        break;
                    }
                    let last_tran = txs.last().unwrap();
                    before_signature = Some(Signature::from_str(&last_tran.signature).unwrap());
                    // finish get history data when last call returns nothing
                    // or first transaction block_slot less then first_slot
                    let finished = last_tran.slot < first_slot;
                    info!(
                        "Got {:?} filtered addresses in {:?}, last address: {:?} in slot {:?}",
                        txs.len(),
                        now.elapsed(),
                        &before_signature,
                        last_tran.slot
                    );
                    for tran in txs {
                        if first_slot <= tran.slot {
                            //Prepend matched transaction hash
                            res_signatures.insert(0, tran.signature);
                        }
                    }
                    if finished {
                        break;
                    }
                }
                Err(err) => {
                    error!("{:?}", &err);
                    break;
                }
            }
        }
        res_signatures
    }
    //Get transactions by hashes from chain and group by block_slot
    pub fn get_confirmed_blocks(&self, signatures: &Vec<String>) -> Vec<ConfirmedBlockWithSlot> {
        let mut start_tx = 0_usize;
        //let mut res_vec = vec![];
        //Group transactions by block
        let mut group_transactions: HashMap<u64, Vec<ConfirmedTransaction>> = HashMap::new();
        while start_tx < signatures.len() {
            let last_index = signatures.len().min(start_tx + TRANSACTION_BATCH_SIZE);
            let params = signatures[start_tx..last_index]
                .iter()
                .map(|tx| json!([tx, "base64"]))
                .collect();
            let res: ClientResult<Vec<ClientResult<EncodedConfirmedTransaction>>> = self
                .rpc_client
                .send_batch(RpcRequest::GetTransaction, params);
            debug!("{:?}", res);
            if let Ok(trans) = res {
                trans
                    .into_iter()
                    .filter_map(|res| res.ok())
                    .for_each(|tran| {
                        if let Some(confirmed_transaction) = Self::decode_transaction(&tran) {
                            group_transactions
                                .entry(tran.slot)
                                .or_insert(vec![])
                                .push(confirmed_transaction);
                        }
                    });
            }
            start_tx += TRANSACTION_BATCH_SIZE;
        }
        group_transactions
            .into_iter()
            .map(|(block_slot, transactions)| {
                let block = ConfirmedBlock {
                    previous_blockhash: Default::default(),
                    blockhash: Default::default(),
                    parent_slot: Default::default(),
                    transactions: transactions
                        .into_iter()
                        .map(|tran| tran.transaction)
                        .collect(),
                    rewards: Default::default(),
                    block_time: Default::default(),
                    block_height: Default::default(),
                };
                ConfirmedBlockWithSlot {
                    block_slot,
                    block: Some(block),
                }
            })
            .collect::<Vec<ConfirmedBlockWithSlot>>()
    }
    fn decode_transaction(
        encode_txs: &EncodedConfirmedTransaction,
    ) -> Option<ConfirmedTransaction> {
        let meta = encode_txs.transaction.meta.as_ref().and_then(|ui_meta| {
            Some(TransactionStatusMeta {
                status: ui_meta.status.clone(),
                rewards: ui_meta.rewards.clone(),
                log_messages: ui_meta.log_messages.clone(),
                fee: ui_meta.fee,
                post_balances: ui_meta.post_balances.clone(),
                pre_balances: ui_meta.pre_balances.clone(),
                inner_instructions: ui_meta.inner_instructions.as_ref().and_then(|instruction| {
                    Some(
                        instruction
                            .iter()
                            .map(|ui_inner_instruction| {
                                Self::to_ui_instructions(ui_inner_instruction)
                            })
                            .collect(),
                    )
                }),
                post_token_balances: ui_meta.post_token_balances.as_ref().and_then(|balances| {
                    Some(
                        balances
                            .iter()
                            .map(|ui_ttb| Self::to_transaction_token_balance(ui_ttb))
                            .collect(),
                    )
                }),
                pre_token_balances: ui_meta.pre_token_balances.as_ref().and_then(|balances| {
                    Some(
                        balances
                            .iter()
                            .map(|ui_ttb| Self::to_transaction_token_balance(ui_ttb))
                            .collect(),
                    )
                }),
            })
        });
        encode_txs
            .transaction
            .transaction
            .decode()
            .and_then(|transaction| {
                Some(ConfirmedTransaction {
                    slot: encode_txs.slot,
                    transaction: TransactionWithStatusMeta { transaction, meta },
                    block_time: encode_txs.block_time.clone(),
                })
            })
    }
    fn decode_encoded_block(encoded_block: EncodedConfirmedBlock) -> ConfirmedBlock {
        ConfirmedBlock {
            rewards: encoded_block.rewards,
            transactions: encoded_block
                .transactions
                .iter()
                .filter_map(|transaction| {
                    let meta = &transaction.meta.as_ref().unwrap();
                    let decoded_transaction = transaction.transaction.decode();
                    let post_token_balances: Option<Vec<TransactionTokenBalance>> =
                        match &meta.post_token_balances {
                            Some(post_token_balances) => Some(
                                post_token_balances
                                    .into_iter()
                                    .map(|ui_ttb| Self::to_transaction_token_balance(ui_ttb))
                                    .collect(),
                            ),
                            None => None,
                        };
                    let pre_token_balances: Option<Vec<TransactionTokenBalance>> =
                        match &meta.pre_token_balances {
                            Some(pre_token_balances) => Some(
                                pre_token_balances
                                    .into_iter()
                                    .map(|ui_ttb| Self::to_transaction_token_balance(ui_ttb))
                                    .collect(),
                            ),
                            None => None,
                        };
                    let inner_instructions: Option<Vec<InnerInstructions>> = Some(
                        meta.inner_instructions
                            .clone()
                            .unwrap()
                            .iter()
                            .map(|ui_inner_instruction| {
                                Self::to_ui_instructions(ui_inner_instruction)
                            })
                            .collect(),
                    );

                    match decoded_transaction {
                        Some(decoded_transaction) => {
                            Some(solana_transaction_status::TransactionWithStatusMeta {
                                meta: Some(TransactionStatusMeta {
                                    status: meta.status.clone(),
                                    rewards: meta.rewards.clone(),
                                    log_messages: meta.log_messages.clone(),
                                    fee: meta.fee,
                                    post_balances: meta.post_balances.clone(),
                                    pre_balances: meta.pre_balances.clone(),
                                    inner_instructions: inner_instructions,
                                    post_token_balances,
                                    pre_token_balances,
                                    // EndTodo
                                }),
                                transaction: decoded_transaction,
                            })
                        }
                        None => None,
                    }
                })
                .collect(),
            block_time: encoded_block.block_time,
            blockhash: encoded_block.blockhash,
            block_height: encoded_block.block_height,
            parent_slot: encoded_block.parent_slot,
            previous_blockhash: encoded_block.previous_blockhash,
        }
    }
    fn to_transaction_token_balance(ui_ttb: &UiTransactionTokenBalance) -> TransactionTokenBalance {
        TransactionTokenBalance {
            account_index: ui_ttb.account_index.clone(),
            mint: ui_ttb.mint.clone(),
            ui_token_amount: ui_ttb.ui_token_amount.clone(),
            owner: "".to_string(),
        }
    }

    fn to_ui_instructions(ui_inner_instruction: &UiInnerInstructions) -> InnerInstructions {
        InnerInstructions {
            index: ui_inner_instruction.index,
            //instructions: compiled_instructions,
            instructions: ui_inner_instruction
                .instructions
                .iter()
                .filter_map(|ui_instruction| {
                    match ui_instruction {
                        Compiled(ui_compiled_instruction) => {
                            Some(solana_program::instruction::CompiledInstruction {
                                program_id_index: ui_compiled_instruction.program_id_index,
                                accounts: ui_compiled_instruction.accounts.clone(),
                                data: bs58::decode(ui_compiled_instruction.data.clone())
                                    .into_vec()
                                    .unwrap(),
                            })
                        }
                        // Todo: need support Parsed(UiParsedInstruction)
                        Parsed(ui_parsed_instruction) => {
                            warn!(
                                "Not support ui_instruction type: {:?}",
                                ui_parsed_instruction
                            );
                            None
                        }
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct SolanaNetworkAdapter {
    pub network: String,
    pub adapter: Arc<SolanaAdapter>,
    pub tx: Option<Sender<BlockInfo>>,
}

impl SolanaNetworkAdapter {
    pub fn new(
        network: String,
        storage: Option<Arc<Box<dyn BlockStorage + Sync + Send>>>,
        config: &ChainConfig,
        tx: Option<Sender<BlockInfo>>,
    ) -> Self {
        SolanaNetworkAdapter {
            network,
            adapter: Arc::new(SolanaAdapter::new(storage, config)),
            tx,
        }
    }

    pub fn get_adapter(&self) -> Arc<SolanaAdapter> {
        self.adapter.clone()
    }
}
#[derive(Clone)]
pub struct SolanaNetworkAdapters {
    pub adapters: Vec<SolanaNetworkAdapter>,
    sender: Option<Sender<BlockInfo>>,
    sem: Arc<Semaphore>,
}

impl SolanaNetworkAdapters {
    pub fn new(
        network: &str,
        storage: Option<Arc<Box<dyn BlockStorage + Sync + Send>>>,
        tx: Option<Sender<BlockInfo>>,
    ) -> Self {
        let mut adapters = Vec::default();
        SOLANA_NETWORKS.iter().for_each(|(_, config)| {
            if config.network.as_str() == network {
                adapters.push(SolanaNetworkAdapter::new(
                    network.to_string(),
                    storage.clone(),
                    config,
                    tx.clone(),
                ));
            }
        });
        SolanaNetworkAdapters {
            adapters,
            sender: tx,
            sem: Arc::new(Semaphore::new(2 * BLOCK_BATCH_SIZE)),
        }
    }
    ///Return 2 different adapters, second one will be used in case of error while get block from first one.
    pub fn get_adapters(&mut self, method: &str) -> Vec<Arc<SolanaAdapter>> {
        self.adapters.sort_by(|a, b| {
            let a_permits = a.adapter.get_available_permits();
            b.adapter.get_available_permits().cmp(&a_permits)
        });
        self.adapters
            .iter()
            .map(|adapter| adapter.adapter.clone())
            .collect::<Vec<Arc<SolanaAdapter>>>()
    }
    ///Get available blocks from [start_slot].
    /// If start_slot is none then result contains current block
    /// otherwise result contains available blocks from start_slot
    pub fn get_block_slots(&mut self, start_slot: Option<Slot>) -> ClientResult<Vec<Slot>> {
        let network_adapters = self.get_adapters("get_block_slots");
        match start_slot {
            None => {
                for adapter in network_adapters {
                    let res = adapter.get_latest_block().and_then(|slot| Ok(vec![slot]));
                    if res.is_ok() {
                        return res;
                    }
                }
            }
            Some(slot) => {
                for adapter in network_adapters {
                    let res = adapter.get_block_slots(slot);
                    if res.is_ok() {
                        return res;
                    }
                }
            }
        }

        Ok(Vec::default())
    }

    pub async fn start(&mut self) {
        let mut last_block: Option<u64> = None;
        loop {
            match self.get_block_slots(last_block) {
                Ok(slots) => {
                    // Root is finalized block in Solana
                    if slots.len() == 0 {
                        log::info!("No pending blocks. Sleep for a while then continue with get available finality blocks");
                        sleep(Duration::from_millis(GET_NEW_SLOT_DELAY_MS)).await;
                        continue;
                    }
                    if self.sender.is_some() {
                        self.sender
                            .as_ref()
                            .unwrap()
                            .send(BlockInfo::from(&slots))
                            .await;
                    }
                    log::info!("Pending blocks {}: {:?}", slots.len(), &slots);
                    //Prepare parameter for next iteration
                    last_block = slots.last().and_then(|val| Some(val + 1));

                    for slot in slots {
                        let adapters = self.get_adapters("get_block");
                        let adapter_counter = adapters.len();
                        if adapter_counter > 0 {
                            //let permit = Arc::clone(&self.sem).acquire_owned().await.unwrap();
                            let permit = adapters.get(0).unwrap().acquire_owned().await.unwrap();
                            let sender = self.sender.clone();
                            tokio::spawn(async move {
                                for (ind, adapter) in adapters.iter().enumerate() {
                                    log::info!(
                                        "Spawn new thread for block {:?} using network {:?}",
                                        &slot,
                                        &adapter.network
                                    );

                                    match timeout(
                                        Duration::from_secs(GET_BLOCK_TIMEOUT_SEC),
                                        adapter.get_block_data(slot),
                                    )
                                    .await
                                    {
                                        Ok(Ok(block)) => {
                                            debug!(
                                                "*** ChainAdapter sending block: {}",
                                                block.block_slot
                                            );
                                            if let Some(tx) = sender.as_ref() {
                                                tx.send(BlockInfo::ConfirmBlockWithSlot(block))
                                                    .await;
                                            }
                                            break;
                                        }
                                        _ => {
                                            if (ind < adapter_counter - 1) {
                                                info!(
                                                    "Retry get data of block {:?} with next adapter",
                                                    &slot
                                                );
                                            } else {
                                                info!(
                                                "Can not get data of the block {:?} from all available node in network",
                                                &slot);
                                                if let Some(tx) = sender.as_ref() {
                                                    tx.send(BlockInfo::ConfirmBlockWithSlot(
                                                        ConfirmedBlockWithSlot {
                                                            block_slot: slot,
                                                            block: None,
                                                        },
                                                    ))
                                                    .await;
                                                }
                                            }
                                        }
                                    }
                                }

                                drop(permit);
                            });
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Get slot error: {:?}", err);
                    sleep(Duration::from_millis(GET_NEW_SLOT_DELAY_MS)).await;
                }
            }
        }
    }
}
impl SolanaNetworkAdapters {
    pub fn get_signatures_for_address(
        &mut self,
        address: &Pubkey,
        first_slot: Option<u64>,
        end_signature: Option<Signature>,
    ) -> Vec<String> {
        let adapters = self.get_adapters("get_signatures_for_address");
        //Todo: add round rubin between adapters
        for adapter in adapters {
            //Using first adapter
            return adapter.get_signatures_for_address(address, first_slot, end_signature);
        }
        Vec::default()
    }
    //Get transactions by hashes from chain and group by block_slot
    pub fn get_confirmed_blocks(
        &mut self,
        signatures: &Vec<String>,
    ) -> Vec<ConfirmedBlockWithSlot> {
        let adapters = self.get_adapters("get_confirmed_blocks");
        //Todo: add round rubin between adapters
        for adapter in adapters {
            //Using first adapter
            return adapter.get_confirmed_blocks(signatures);
        }
        Vec::default()
    }
}
pub struct RuntimeAdapter {
    pub sol_adapters: Arc<SolanaNetworkAdapters>,
}
impl bc::RuntimeAdapter<Chain> for RuntimeAdapter {
    fn host_fns(&self, ds: &DataSource) -> Result<Vec<HostFn>, Error> {
        todo!()
    }
}
