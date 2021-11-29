use crate::command::ChainConfig;
use crate::stream_service::BlockInfo;
use chain_solana::types::ConfirmedBlockWithSlot;
use log::{info, warn};
use massbit::firehose::bstream::{BlockRequest, BlockResponse};
use massbit::prelude::{Arc, Duration};
use massbit_chain_solana::data_type::{ExtBlock, SolanaBlock};
use massbit_common::prelude::tokio::sync::{OwnedSemaphorePermit, Semaphore};
use massbit_common::prelude::tokio::time::{sleep, timeout};
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::UiInstruction::{Compiled, Parsed};
use solana_transaction_status::{
    ConfirmedBlock, EncodedConfirmedBlock, InnerInstructions, TransactionStatusMeta,
    TransactionTokenBalance, UiInnerInstructions, UiTransactionEncoding, UiTransactionTokenBalance,
};
use std::error::Error;
use std::time::Instant;
use tokio::sync::mpsc::{Receiver, Sender};

const VERSION: &str = "1.7.0";
const BLOCK_AVAILABLE_MARGIN: u64 = 100;
const RPC_BLOCK_ENCODING: UiTransactionEncoding = UiTransactionEncoding::Base64;
const GET_BLOCK_TIMEOUT_SEC: u64 = 60;
const BLOCK_BATCH_SIZE: u64 = 10;
const GET_NEW_SLOT_DELAY_MS: u64 = 500;
const TRANSACTION_BATCH_SIZE: usize = 100;
// The max value is 1000
const LIMIT_FILTER_RESULT: usize = 1000;

pub struct ChainAdapter {
    client: Arc<RpcClient>,
    sem: Arc<Semaphore>,
    sender: Sender<BlockInfo>,
    last_block: Option<u64>,
}
impl ChainAdapter {
    pub fn new(config: &ChainConfig, sender: Sender<BlockInfo>) -> Self {
        info!("Init Solana client with url: {:?}", &config.url);
        let client = Arc::new(RpcClient::new(config.url.clone()));
        info!("Finished init Solana client");
        ChainAdapter {
            client,
            sem: Arc::new(Semaphore::new(2 * BLOCK_BATCH_SIZE as usize)),
            sender,
            last_block: None,
        }
    }
    pub async fn start(&mut self) {
        loop {
            match self.client.get_slot() {
                Ok(new_slot) => {
                    // Root is finalized block in Solana
                    let current_root = new_slot - BLOCK_AVAILABLE_MARGIN;
                    //Send current slot to broadcaster
                    self.sender.send(BlockInfo::from(current_root));
                    //info!("Root: {:?}",new_info.root);
                    match self.last_block {
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
                            let number_get_slot =
                                (current_root - value_last_indexed_slot).min(BLOCK_BATCH_SIZE);
                            let block_range = value_last_indexed_slot
                                ..(value_last_indexed_slot + number_get_slot);

                            for block_slot in block_range {
                                let permit = Arc::clone(&self.sem).acquire_owned().await.unwrap();
                                let client = self.client.clone();
                                let sender = self.sender.clone();
                                tokio::spawn(async move {
                                    match timeout(
                                        Duration::from_secs(GET_BLOCK_TIMEOUT_SEC),
                                        Self::get_block(client, permit, block_slot),
                                    )
                                    .await
                                    {
                                        Ok(res) => {
                                            info!(
                                                "Finish tokio::spawn for getting block number: {:?}",
                                                &block_slot
                                            );
                                            if let Ok(block) = res {
                                                sender.send(BlockInfo::ConfirmBlockWithSlot(block));
                                            }
                                        }
                                        Err(err) => {
                                            warn!(
                                                "get_block timed out at block number {}",
                                                &block_slot
                                            );
                                        }
                                    }
                                });
                            }
                            self.last_block = Some(value_last_indexed_slot + number_get_slot);
                        }
                        _ => self.last_block = Some(current_root),
                    };
                }
                Err(err) => {
                    eprintln!("Get slot error: {:?}", err);
                    sleep(Duration::from_millis(GET_NEW_SLOT_DELAY_MS)).await;
                    continue;
                }
            }
        }
    }
    async fn get_block(
        client: Arc<RpcClient>,
        permit: OwnedSemaphorePermit,
        block_number: u64,
    ) -> Result<ConfirmedBlockWithSlot, Box<dyn Error + Send + Sync + 'static>> {
        let _permit = permit;
        info!("Starting RPC get Block {}", block_number);
        let now = Instant::now();
        let block = client.get_block_with_encoding(block_number, RPC_BLOCK_ENCODING);
        let elapsed = now.elapsed();
        match block {
            Ok(block) => {
                info!(
                    "Finished RPC get Block: {:?}, time: {:?}, hash: {}",
                    block_number, elapsed, &block.blockhash
                );
                Ok(ConfirmedBlockWithSlot {
                    block_slot: block_number,
                    block: Some(Self::decode_encoded_block(block)),
                })
            }
            _ => {
                info!(
                    "Cannot get RPC get Block: {:?}, Error:{:?}, time: {:?}",
                    block_number, block, elapsed
                );
                //Err(format!("Error cannot get block").into())
                //Todo: implement retry get missing block
                //Send None to broadcaster -
                Ok(ConfirmedBlockWithSlot {
                    block_slot: block_number,
                    block: None,
                })
            }
        }
    }
    // fn to_generic_block(
    //     &self,
    //     blocks_with_number: Vec<(solana_transaction_status::ConfirmedBlock, u64)>,
    // ) -> BlockResponse {
    //     let ext_blocks: Vec<ConfirmedBlock> = blocks_with_number
    //         .into_iter()
    //         .map(|(block, block_number)| {
    //             let timestamp = (&block).block_time.unwrap_or_default();
    //             let list_log_messages = block
    //                 .transactions
    //                 .iter()
    //                 .map(|transaction| transaction.meta.as_ref().unwrap().log_messages.clone())
    //                 .collect();
    //             ExtBlock {
    //                 version: VERSION.to_string(),
    //                 block,
    //                 block_number,
    //                 timestamp,
    //                 list_log_messages,
    //             }
    //         })
    //         .collect();
    //     BlockResponse {
    //         version: VERSION.to_string(),
    //         payload: serde_json::to_vec(&ext_blocks).unwrap(),
    //     }
    // }
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
                                Self::to_ui_instructions(ui_inner_instruction.clone())
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
    fn to_ui_instructions(ui_inner_instruction: UiInnerInstructions) -> InnerInstructions {
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
