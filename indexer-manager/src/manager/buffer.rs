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
    last_reading_blocks: HashMap<String, Slot>,
}

impl IncomingBlocks {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: RwLock::new(VecDeque::new()),
            last_reading_blocks: Default::default(),
        }
    }
    pub fn append_blocks(&mut self, blocks: Vec<SolanaBlock>) {
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
    pub fn read_blocks(&mut self, indexer: &String) -> Vec<Arc<SolanaBlock>> {
        let last_slot = self.last_reading_blocks.get(indexer);
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
        if blocks.len() > 0 {
            self.last_reading_blocks
                .insert(indexer.clone(), blocks.iter().last().unwrap().block_number);
        }
        blocks
    }
}
