use massbit_solana_sdk::types::{ExtBlock, SolanaBlock};
use solana_sdk::clock::Slot;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

/// Buffer for storage incoming block from chain reader for each smart contract address
/// Buffer is shared for one writing and multiple reading threads
/// In order to limit resource, this buffer is inited with fixed capacity for example 1024 elements.
///
pub struct IncomingBlocks {
    capacity: usize,
    buffer: RwLock<VecDeque<Arc<SolanaBlock>>>,
}

impl IncomingBlocks {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: RwLock::new(VecDeque::new()),
        }
    }
    pub fn append_blocks(&self, blocks: Vec<SolanaBlock>) {
        let mut write_lock = self.buffer.write().unwrap();
        while write_lock.len() >= self.capacity - blocks.len() {
            //First cycle to fill buffer - just append into end of vector
            write_lock.pop_front();
        }
        for block in blocks.into_iter() {
            write_lock.push_back(Arc::new(block));
        }
    }
    /// Read all unprocessed blocks (blocks with indexes from next_reading_index to self.latest_index) in buffer for indexer
    /// Input: indexer hash
    pub fn read_blocks(&self, last_slot: &Option<Slot>) -> Vec<Arc<SolanaBlock>> {
        let mut read_lock = self.buffer.read().unwrap();
        let blocks = match last_slot {
            None => read_lock
                .iter()
                .map(|block| block.clone())
                .collect::<Vec<Arc<SolanaBlock>>>(),
            Some(slot) => read_lock
                .iter()
                .filter(|block| block.block_number > *slot)
                .map(|block| block.clone())
                .collect::<Vec<Arc<SolanaBlock>>>(),
        };
        blocks
    }
}
