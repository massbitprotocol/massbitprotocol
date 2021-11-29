use serde::{Deserialize, Serialize};
use solana_transaction_status::ConfirmedBlock;

pub type BlockSlot = i32;
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
