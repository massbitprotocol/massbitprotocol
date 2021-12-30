use std::path::Path;
extern crate db_key as key;
extern crate leveldb;
use crate::storage::BlockStorage;
use key::Key;
use leveldb::database::Database;
use leveldb::error::Error;
use leveldb::iterator::Iterable;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use log::log;
use solana_program::clock::Slot;

pub struct BlockKey {
    slot: u64,
}

impl Key for BlockKey {
    fn from_u8(key: &[u8]) -> Self {
        assert!(key.len() == 8);
        let mut slot: u64 = key[0] as u64;
        for i in 1..8 {
            slot = slot << 8 | key[i] as u64;
        }
        BlockKey { slot }
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        let mut dst = [0u8, 0, 0, 0, 0, 0, 0, 0];
        for i in 0..8 {
            dst[i] = (self.slot >> (8 * (7 - i))) as u8;
        }
        f(&dst)
    }
}

impl From<Slot> for BlockKey {
    fn from(slot: Slot) -> Self {
        BlockKey { slot }
    }
}
pub struct LevelDBStorage {
    database: Database<BlockKey>,
}

impl LevelDBStorage {
    pub fn new(db_dir_path: &str) -> Self {
        let mut options = Options::new();
        let db_path = Path::new(db_dir_path);
        options.create_if_missing = true;
        let database = match Database::open(&db_path, options) {
            Ok(db) => db,
            Err(e) => {
                panic!("failed to open database: {:?}", e)
            }
        };
        Self { database }
    }
    pub fn remove_old_blocks(&self) {}
}
impl BlockStorage for LevelDBStorage {
    fn store_block(&self, block_slot: Slot, content: &[u8]) {
        let write_opts = WriteOptions::new();
        match self
            .database
            .put(write_opts, BlockKey::from(block_slot), content)
        {
            Ok(_) => {}
            Err(err) => {
                log::error!("failed to write to database: {:?}", &err);
            }
        };
        //Todo: check if storage size exceed some limit then remove old blocks
        self.remove_old_blocks();
    }

    fn get_block(&self, block_slot: Slot) -> Option<Vec<u8>> {
        let read_opts = ReadOptions::new();
        match self.database.get(read_opts, BlockKey::from(block_slot)) {
            Ok(res) => res,
            Err(err) => {
                log::error!(
                    "failed to get block {:?} from database: {:?}",
                    block_slot,
                    &err
                );
                None
            }
        }
    }
}
