use massbit_solana_sdk::types::{ExtBlock, SolanaBlock};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Buffer for storage incoming block from chain reader for each smart contract address
/// Buffer is shared for one writing and multiple reading threads
/// In order to limit resource, this buffer is inited with fixed capacity for example 1024 elements.
/// It stores next index for writing thead and indexes for every readers
/// New received block is insert into buffer at next_writing_index
/// When next_writing_index reach the end of buffer vector than it is reset to the head of the vector
/// If some next_reading element overwritten then next_reading is increasing by 1
/// i.e correspond indexer misses overwritten block
pub struct IncomingBlocks {
    capacity: usize,
    buffer: RwLock<Vec<Arc<SolanaBlock>>>,
    oldest_index: usize,
    latest_index: usize,
    next_reading_indexes: HashMap<String, usize>,
}

impl IncomingBlocks {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: RwLock::new(Vec::new()),
            oldest_index: 0,
            latest_index: 0,
            next_reading_indexes: Default::default(),
        }
    }
    pub fn append_block(&mut self, block: SolanaBlock) {
        let mut write_lock = self.buffer.write().unwrap();
        if write_lock.len() < self.capacity {
            //First cycle to fill buffer - just append into end of vector
            write_lock.push(Arc::new(block));
        } else {
            //Buffer if full, overwrite oldest block
            let mut index = self.latest_index + 1;
            if index == self.capacity {
                index = 0;
            }
            let value = std::mem::replace(&mut write_lock[index], Arc::new(block));
            self.latest_index = index;
            //recalculate oldest index
            if index < self.capacity - 1 {
                self.oldest_index = index + 1;
            } else {
                self.oldest_index = 0;
            }
            //drop(write_lock);
        }
    }
    pub fn read_block(&mut self, indexer: String) -> Option<Arc<SolanaBlock>> {
        let mut read_lock = self.buffer.read().unwrap();
        let mut index = self
            .next_reading_indexes
            .get(&indexer)
            .unwrap_or(&self.oldest_index)
            .clone();
        //update reading index for next call
        if index == self.capacity - 1 {
            self.next_reading_indexes.insert(indexer, index + 1);
        } else {
            self.next_reading_indexes.insert(indexer, 0);
        }
        read_lock.get(index).and_then(|arc| Some(arc.clone()))
    }
}
