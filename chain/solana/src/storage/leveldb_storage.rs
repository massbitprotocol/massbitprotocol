use anyhow::anyhow;
use std::path::Path;
use std::time::Instant;

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
use massbit_common::prelude::serde_json;
use solana_program::clock::Slot;
use solana_transaction_status::EncodedConfirmedBlock;
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
    fn store_block(
        &self,
        block_slot: Slot,
        block: &EncodedConfirmedBlock,
    ) -> Result<(), anyhow::Error> {
        let write_opts = WriteOptions::new();
        let now = Instant::now();
        let res = serde_json::to_vec(block)
            .map_err(|err| {
                log::error!("failed to write to database: {:?}", &err);
                anyhow!("{:?}", &err)
            })
            .and_then(|content| {
                self.database
                    .put(write_opts, BlockKey::from(block_slot), content.as_slice())
                    .map_err(|err| {
                        log::error!("failed to write to database: {:?}", &err);
                        anyhow!("{:?}", &err)
                    })
            });
        log::info!("Store block in leveldb in {:?}", now.elapsed());
        //Todo: check if storage size exceed some limit then remove old blocks
        self.remove_old_blocks();
        res
    }

    fn get_block(&self, block_slot: Slot) -> Option<EncodedConfirmedBlock> {
        let read_opts = ReadOptions::new();
        match self.database.get(read_opts, BlockKey::from(block_slot)) {
            Ok(res) => res.and_then(|content| {
                let block: Option<EncodedConfirmedBlock> =
                    serde_json::from_slice(content.as_slice()).ok();
                block
            }),
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
