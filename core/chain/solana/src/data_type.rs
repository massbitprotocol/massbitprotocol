use solana_transaction_status;
use std::error::Error;
use serde_json;
use serde::{Deserialize, Serialize};
use solana_transaction_status::{TransactionStatusMeta, TransactionTokenBalance, UiTransactionTokenBalance};
use std::rc::Rc;
use std::sync::Arc;
use log::info;


//***************** Solana data type *****************
// EncodedConfirmedBlock is block with vec of EncodedTransactionWithStatusMeta.
pub type SolanaEncodedBlock = ExtEncodedBlock;
pub type SolanaBlock = ExtBlock;
pub type SolanaTransaction = ExtTransaction;
// The most similar Event concept in Solana is log_messages in UiTransactionStatusMeta in EncodedTransactionWithStatusMeta
pub type SolanaLogMessages = ExtLogMessages;
//***************** End solana data type *****************


type Number = u32;
type Date = i64;
type LogMessages = Option<Vec<String>>;
type Transaction = solana_transaction_status::TransactionWithStatusMeta;
type EncodedBlock = solana_transaction_status::EncodedConfirmedBlock;
type Block = solana_transaction_status::ConfirmedBlock;
// type Hash = String;

pub fn decode(payload: &mut Vec<u8>) -> Result<SolanaEncodedBlock, Box<dyn Error>>
{
    let decode_block: SolanaEncodedBlock = serde_json::from_slice(&payload).unwrap();
    Ok(decode_block)
}

pub fn get_list_log_messages_from_encoded_block(block: &EncodedBlock) -> Vec<LogMessages> {
    block.transactions.iter()
        .map(|transaction| {
            transaction.meta.as_ref().unwrap().log_messages.clone()
        })
        .collect()
}

fn UiTransactionTokenBalance_to_TransactionTokenBalance(ui_ttb: &UiTransactionTokenBalance)-> TransactionTokenBalance{
    TransactionTokenBalance {
        account_index: ui_ttb.account_index.clone(),
        mint: ui_ttb.mint.clone(),
        ui_token_amount: ui_ttb.ui_token_amount.clone(),
    }
}

pub fn decode_encoded_block (encoded_block: EncodedBlock) -> Block {
    Block {
        rewards: encoded_block.rewards,
        transactions: encoded_block.transactions.iter().filter_map(|transaction| {
            let meta = &transaction.meta.as_ref().unwrap();
            let decoded_transaction = transaction.transaction.decode();
            let post_token_balances: Option<Vec<TransactionTokenBalance>> = match &meta.post_token_balances {
                Some(post_token_balances) => {
                    Some(post_token_balances.into_iter()
                        .map(|ui_ttb| UiTransactionTokenBalance_to_TransactionTokenBalance(ui_ttb))
                        .collect())
                },
                None => None
            };
            let pre_token_balances: Option<Vec<TransactionTokenBalance>> = match &meta.pre_token_balances {
                Some(pre_token_balances) => {
                    Some(pre_token_balances.into_iter()
                        .map(|ui_ttb| UiTransactionTokenBalance_to_TransactionTokenBalance(ui_ttb))
                        .collect())
                },
                None => None
            };
            // let inner_instructions = match &meta.inner_instructions {
            //     Some(inner_instructions) => Some(inner_instructions),
            //     None => None
            // }
            info!("inner_instructions: {:#?}", &meta.inner_instructions);
            //println!("*** Decode transaction: {:?}",decoded_transaction);
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
                            // Todo: decode the following field from UiTransactionStatusMeta, now just ignore
                            inner_instructions: None,
                            post_token_balances,
                            pre_token_balances,
                            // EndTodo
                        }),
                        transaction: decoded_transaction,
                    })
                },
                None => None,
            }
        }).collect(),
        block_time: encoded_block.block_time,
        blockhash: encoded_block.blockhash,
        block_height: encoded_block.block_height,
        parent_slot: encoded_block.parent_slot,
        previous_blockhash: encoded_block.previous_blockhash,
    }
}

pub fn convert_solana_encoded_block_to_solana_block (encoded_block: SolanaEncodedBlock) -> SolanaBlock {
    SolanaBlock {
        version: encoded_block.version,
        timestamp: encoded_block.timestamp,
        block: decode_encoded_block(encoded_block.block),
        list_log_messages: encoded_block.list_log_messages,
    }
}

// Similar to
// https://github.com/subquery/subql/blob/93afc96d7ee0ff56d4dd62d8a145088f5bb5e3ec/packages/types/src/interfaces.ts#L18
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ExtEncodedBlock {
    pub version: String,
    pub timestamp: Date,
    pub block: EncodedBlock,
    pub list_log_messages: Vec<LogMessages>,
}


#[derive(PartialEq, Clone,  Debug, Serialize, Deserialize)]
pub struct ExtBlock {
    pub version: String,
    pub timestamp: Date,
    pub block: Block,
    pub list_log_messages: Vec<LogMessages>,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ExtTransaction {
    pub block_number: Number,
    pub transaction: Transaction,
    //pub block: Arc<ExtBlock>,
    pub log_messages: LogMessages,
    pub success: bool,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ExtLogMessages {
    pub block_number: Number,
    pub log_messages: LogMessages,
    pub transaction: Transaction,
    //pub block: Arc<ExtBlock>,
}

