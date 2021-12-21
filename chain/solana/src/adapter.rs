use super::types::ChainConfig;
use crate::chain::Chain;
use crate::data_source::DataSource;
use crate::types::{BlockInfo, ConfirmedBlockWithSlot, Pubkey};
use crate::{LIMIT_FILTER_RESULT, SOLANA_NETWORKS, TRANSACTION_BATCH_SIZE};
use log::{debug, error, info, warn};
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
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc::Sender;
use tokio::sync::Semaphore;
use tokio::time::{sleep, timeout};

const BLOCK_AVAILABLE_MARGIN: u64 = 100;
const GET_NEW_SLOT_DELAY_MS: u64 = 500;
const BLOCK_BATCH_SIZE: usize = 10;
const GET_BLOCK_TIMEOUT_SEC: u64 = 60;
const RPC_BLOCK_ENCODING: UiTransactionEncoding = UiTransactionEncoding::Base64;
#[derive(Clone)]
pub struct SolanaAdapter {
    pub rpc_client: Arc<RpcClient>,
    network: String,
}

impl SolanaAdapter {
    pub fn new(config: &ChainConfig) -> Self {
        info!("Init Solana client with url: {:?}", &config.url);
        let rpc_client = Arc::new(RpcClient::new(config.url.clone()));
        info!("Finished init Solana client");
        SolanaAdapter {
            rpc_client,
            network: config.name.clone(),
        }
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
    pub tx: Sender<BlockInfo>,
}

impl SolanaNetworkAdapter {
    pub fn new(network: String, config: &ChainConfig, tx: Sender<BlockInfo>) -> Self {
        SolanaNetworkAdapter {
            network,
            adapter: Arc::new(SolanaAdapter::new(config)),
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
    sender: Sender<BlockInfo>,
    ///Map contains adapter for next call.
    next_adapter: HashMap<String, usize>,
    sem: Arc<Semaphore>,
}

impl SolanaNetworkAdapters {
    pub fn new(network: &str, tx: Sender<BlockInfo>) -> Self {
        let mut adapters = Vec::default();
        SOLANA_NETWORKS.iter().for_each(|(_, config)| {
            if config.network.as_str() == network {
                adapters.push(SolanaNetworkAdapter::new(
                    network.to_string(),
                    config,
                    tx.clone(),
                ));
            }
        });
        SolanaNetworkAdapters {
            adapters,
            sender: tx,
            next_adapter: Default::default(),
            sem: Arc::new(Semaphore::new(2 * BLOCK_BATCH_SIZE)),
        }
    }
    ///Return 2 different adapters, second one will be used in case of error while get block from first one.
    fn get_adapters(&mut self, method: &str) -> Vec<Option<Arc<SolanaAdapter>>> {
        let ind = match self.next_adapter.get(method) {
            Some(val) => val.clone(),
            None => 0,
        };
        let adapter_counter = self.adapters.len();
        self.next_adapter
            .insert(method.to_string(), (ind + 1) % adapter_counter);
        let first_adapter = self
            .adapters
            .get(ind)
            .and_then(|adapter| Some(adapter.adapter.clone()));
        let second_adapter = self
            .adapters
            .get((ind + 1) % adapter_counter)
            .and_then(|adapter| Some(adapter.adapter.clone()));
        vec![first_adapter, second_adapter]
    }
    ///Get available blocks from [start_slot].
    /// If start_slot is none then result contains current block
    /// otherwise result contains available blocks from start_slot
    pub fn get_block_slots(&mut self, start_slot: Option<Slot>) -> ClientResult<Vec<Slot>> {
        let network_adapters = self.get_adapters("get_block_slots");
        match start_slot {
            None => {
                for sol_adapter in network_adapters {
                    if let Some(adapter) = sol_adapter {
                        let res = adapter.get_latest_block().and_then(|slot| Ok(vec![slot]));
                        if res.is_ok() {
                            return res;
                        }
                    }
                }
            }
            Some(slot) => {
                for sol_adapter in network_adapters {
                    if let Some(adapter) = sol_adapter {
                        let res = adapter.get_block_slots(slot);
                        if res.is_ok() {
                            return res;
                        }
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
                        log::info!("No pending blocks. Sleep for awhile then continue with get available finality blocks");
                        sleep(Duration::from_millis(GET_NEW_SLOT_DELAY_MS)).await;
                        continue;
                    }
                    if last_block.is_none() && slots.len() == 1 {
                        let current_block = slots.get(0).unwrap().clone();
                        log::info!("Current block {:?}", &current_block);
                        self.sender.send(BlockInfo::from(current_block)).await;
                    }
                    log::info!("Pending blocks {:?}", &slots);
                    //Prepare parameter for next iteration
                    last_block = slots.last().and_then(|val| Some(val + 1));

                    for slot in slots {
                        let adapters = self
                            .get_adapters("get_block")
                            .iter()
                            .map(|item| item.as_ref().and_then(|adapter| Some(adapter.clone())))
                            .collect::<Vec<Option<Arc<SolanaAdapter>>>>();

                        let permit = Arc::clone(&self.sem).acquire_owned().await.unwrap();
                        let sender = self.sender.clone();
                        tokio::spawn(async move {
                            let adapter_counter = adapters.len();
                            for (ind, sol_adapter) in adapters.iter().enumerate() {
                                if let Some(adapter) = sol_adapter {
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
                                            sender
                                                .send(BlockInfo::ConfirmBlockWithSlot(block))
                                                .await;
                                            break;
                                        }
                                        Ok(Err(err)) => {
                                            if (ind < adapter_counter - 1) {
                                                info!(
                                                "Retry get data of block {:?} with next adapter",
                                                &slot
                                            );
                                            } else {
                                                info!(
                                                "Can not get data of the block {:?} from all available node in network",
                                                &slot);
                                                sender
                                                    .send(BlockInfo::ConfirmBlockWithSlot(
                                                        ConfirmedBlockWithSlot {
                                                            block_slot: slot,
                                                            block: None,
                                                        },
                                                    ))
                                                    .await;
                                            }
                                        }
                                        Err(err) => {
                                            warn!("get_block timed out at block number {}. Retry with second adapter", &slot);
                                        }
                                    }
                                }
                            }

                            drop(permit);
                        });
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
pub struct RuntimeAdapter {
    pub sol_adapters: Arc<SolanaNetworkAdapters>,
}
impl bc::RuntimeAdapter<Chain> for RuntimeAdapter {
    fn host_fns(&self, ds: &DataSource) -> Result<Vec<HostFn>, Error> {
        todo!()
    }
}
