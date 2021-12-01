use super::types::ChainConfig;
use crate::chain::Chain;
use crate::data_source::DataSource;
use crate::types::{ConfirmedBlockWithSlot, Pubkey};
use crate::{LIMIT_FILTER_RESULT, TRANSACTION_BATCH_SIZE};
use log::{debug, error, info, warn};
use massbit::blockchain as bc;
use massbit::blockchain::HostFn;
use massbit::prelude::*;
use serde_json::json;
use solana_client::client_error::Result as ClientResult;
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_client::rpc_request::RpcRequest;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiInstruction::{Compiled, Parsed};
use solana_transaction_status::{
    ConfirmedBlock, ConfirmedTransaction, EncodedConfirmedTransaction, InnerInstructions,
    TransactionStatusMeta, TransactionTokenBalance, TransactionWithStatusMeta, UiInnerInstructions,
    UiTransactionTokenBalance,
};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

#[derive(Clone)]
pub struct SolanaAdapter {
    pub rpc_client: Arc<RpcClient>,
}

impl SolanaAdapter {
    pub fn new(config: &ChainConfig) -> Self {
        info!("Init Solana client with url: {:?}", &config.url);
        let rpc_client = Arc::new(RpcClient::new(config.url.clone()));
        info!("Finished init Solana client");
        SolanaAdapter { rpc_client }
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
}

impl SolanaNetworkAdapter {
    pub fn new(network: String, config: &ChainConfig) -> Self {
        SolanaNetworkAdapter {
            network,
            adapter: Arc::new(SolanaAdapter::new(config)),
        }
    }
    pub fn from(network: String) -> Self {
        let config = super::SOLANA_NETWORKS
            .get(network.as_str())
            .or(super::SOLANA_NETWORKS.get("mainnet"))
            .unwrap();
        SolanaNetworkAdapter {
            network,
            adapter: Arc::new(SolanaAdapter::new(config)),
        }
    }
    pub fn get_adapter(&self) -> Arc<SolanaAdapter> {
        self.adapter.clone()
    }
}
#[derive(Clone)]
pub struct SolanaNetworkAdapters {
    pub adapters: Vec<SolanaNetworkAdapter>,
}

pub struct RuntimeAdapter {
    pub sol_adapters: Arc<SolanaNetworkAdapters>,
}
impl bc::RuntimeAdapter<Chain> for RuntimeAdapter {
    fn host_fns(&self, ds: &DataSource) -> Result<Vec<HostFn>, Error> {
        todo!()
    }
}
