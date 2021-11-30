use serde::{Deserialize, Serialize};
use solana_transaction_status::TransactionWithStatusMeta;
use std::str::FromStr;

pub type SolanaBlock = ExtBlock;
pub type SolanaTransaction = ExtTransaction;
// The most similar Event concept in Solana is log_messages in UiTransactionStatusMeta in EncodedTransactionWithStatusMeta
pub type SolanaLogMessages = ExtLogMessages;
pub type Pubkey = solana_program::pubkey::Pubkey;
type Block = solana_transaction_status::ConfirmedBlock;
type Transaction = solana_transaction_status::TransactionWithStatusMeta;
type LogMessages = Option<Vec<String>>;

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ExtBlock {
    pub version: String,
    pub timestamp: i64,
    //Todo: rename this field to block_slot
    pub block_number: u64,
    pub block: Block,
    pub list_log_messages: Vec<LogMessages>,
}
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ExtTransaction {
    //Todo: rename this field to block_slot
    pub block_number: u64,
    pub transaction: Transaction,
    //pub block: Arc<ExtBlock>,
    pub log_messages: LogMessages,
    pub success: bool,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct ExtLogMessages {
    //Todo: rename this field to block_slot
    pub block_number: u64,
    pub log_messages: LogMessages,
    pub transaction: Transaction,
    //pub block: Arc<ExtBlock>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SolanaFilter {
    pub keys: Vec<Pubkey>,
}
impl SolanaFilter {
    pub fn new(keys: Vec<&str>) -> Self {
        SolanaFilter {
            keys: keys
                .iter()
                .map(|key| Pubkey::from_str(key).unwrap_or_default())
                .collect(),
        }
    }
    fn is_match(&self, tran: &TransactionWithStatusMeta) -> bool {
        self.keys.iter().any(|key| {
            tran.transaction
                .message
                .account_keys
                .iter()
                .any(|account_key| key == account_key)
        })
    }

    pub fn filter_block(&self, block: Block) -> Block {
        // If there are no key, then accept all transactions
        if self.keys.is_empty() {
            return block;
        }
        let mut filtered_block = block.clone();
        filtered_block.transactions = block
            .transactions
            .into_iter()
            .filter_map(|tran| {
                if self.is_match(&tran) {
                    Some(tran)
                } else {
                    None
                }
            })
            .collect();
        filtered_block
    }
}
