use serde::{Deserialize, Serialize};
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;
use solana_sdk::signature::Signature;
use solana_transaction_status::{ConfirmedBlock, TransactionWithStatusMeta};
use std::str::FromStr;

pub type Pubkey = solana_program::pubkey::Pubkey;
pub type BlockSlot = i64;

#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub url: String,
    pub ws: String,
    pub network: String,
    pub supports_eip_1898: bool,
}

/// A block hash and block number from a specific block.
///
/// Block numbers are signed 32 bit integers
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockPtr {
    pub hash: String,
    pub number: BlockSlot,
}
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ConfirmedBlockWithSlot {
    pub block_slot: u64,
    pub block: Option<ConfirmedBlock>,
}

impl ConfirmedBlockWithSlot {
    pub fn cheap_clone(&self) -> Self {
        let ConfirmedBlockWithSlot { block_slot, block } = self;
        ConfirmedBlockWithSlot {
            block_slot: *block_slot,
            block: block.as_ref().and_then(|block| {
                Some(ConfirmedBlock {
                    previous_blockhash: block.previous_blockhash.clone(),
                    blockhash: block.blockhash.clone(),
                    parent_slot: block.parent_slot.clone(),
                    transactions: vec![],
                    rewards: block.rewards.clone(),
                    block_time: block.block_time.clone(),
                    block_height: block.block_height.clone(),
                })
            }),
        }
    }
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

    pub fn filter_block(&self, block: ConfirmedBlock) -> ConfirmedBlock {
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

#[derive(Debug)]
pub struct ResultFilterTransaction {
    pub txs: Vec<RpcConfirmedTransactionStatusWithSignature>,
    pub last_tx_signature: Option<Signature>,
    pub finished: bool,
}
