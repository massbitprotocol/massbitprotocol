use crate::storage::BlockStorage;
use std::path::Path;
extern crate leveldb;

use leveldb::database::Database;
use leveldb::iterator::Iterable;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};

pub struct BlockKey {
    
}

impl Key fo {
    
}
pub struct LevelDBStorage {
    database: Database<ii3232>,
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
}
impl BlockStorage for LevelDBStorage {
    fn store_block(&self, block_slot: , content: &[u8]) {
        todo!()
    }

    fn get_block(&self, block_slot: u64) -> Vec<u8> {
        todo!()
    }
}
