mod leveldb_storage;

pub trait BlockStorage {
    fn store_block(&self, block_slot: u64, content: &[u8]);
    fn get_block(&self, block_slot: u64) -> Option<Vec<u8>>;
}
