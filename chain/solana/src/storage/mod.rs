pub mod leveldb_storage;

pub use leveldb_storage::*;
use solana_transaction_status::EncodedConfirmedBlock;
pub trait BlockStorage {
    fn store_block(
        &self,
        block_slot: u64,
        content: &EncodedConfirmedBlock,
    ) -> Result<(), anyhow::Error>;
    fn get_block(&self, block_slot: u64) -> Option<EncodedConfirmedBlock>;
}
