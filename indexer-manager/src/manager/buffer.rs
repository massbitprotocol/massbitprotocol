use crate::WAITING_FOR_BUFFER_WRITING_MICROSECOND;
use massbit_solana_sdk::types::{ExtBlock, SolanaBlock};
use solana_sdk::clock::Slot;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Buffer for storage incoming block from chain reader for each smart contract address
/// Buffer is shared for one writing and multiple reading threads
/// In order to limit resource, this buffer is inited with fixed capacity for example 1024 elements.
///
pub struct IncomingBlocks {
    capacity: usize,
    reading_flag: AtomicBool,
    buffer: RwLock<VecDeque<Arc<SolanaBlock>>>,
}

impl IncomingBlocks {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            reading_flag: AtomicBool::new(true),
            buffer: RwLock::new(VecDeque::new()),
        }
    }
    pub fn append_blocks(&self, blocks: Vec<SolanaBlock>) {
        let now = Instant::now();
        //Prevent other thread acquires lock for reading
        self.reading_flag.store(false, Ordering::Relaxed);
        let mut write_lock = self.buffer.write().unwrap();
        log::info!(
            "Waiting time {:?} for append {} blocks: {:?} into buffer with current size: {}",
            now.elapsed(),
            blocks.len(),
            blocks
                .iter()
                .map(|block| block.block_number)
                .collect::<Vec<Slot>>(),
            write_lock.len()
        );
        let now = Instant::now();
        let total_size = write_lock.len() + blocks.len();
        for _ in self.capacity..total_size {
            //First cycle to fill buffer - just append into end of vector
            write_lock.pop_front();
        }
        for block in blocks.into_iter() {
            write_lock.push_back(Arc::new(block));
        }
        self.reading_flag.store(true, Ordering::Relaxed);
        log::info!("Writing time {:?}", now.elapsed());
    }
    /// Read all unprocessed blocks (blocks with indexes from next_reading_index to self.latest_index) in buffer for indexer
    /// Input: indexer hash
    pub fn read_blocks(&self, last_slot: &Option<Slot>) -> Vec<Arc<SolanaBlock>> {
        let now = Instant::now();
        loop {
            let readable = self.reading_flag.load(Ordering::Relaxed);
            if readable {
                break;
            }
            //Time to lock buffer for writing is about 7-8 micro_seconds
            sleep(Duration::from_micros(
                WAITING_FOR_BUFFER_WRITING_MICROSECOND,
            ))
        }
        let mut read_lock = self.buffer.read().unwrap();
        let blocks = match last_slot {
            None => read_lock
                .iter()
                .map(|block| block.clone())
                .collect::<Vec<Arc<SolanaBlock>>>(),
            Some(slot) => {
                let mut ind = read_lock.len() - 1;
                let mut blocks = Vec::default();
                while ind >= 0 {
                    let block = read_lock.get(ind).unwrap();
                    if block.block_number > *slot {
                        blocks.insert(0, block.clone());
                    } else {
                        break;
                    }
                    ind = ind - 1;
                }
                blocks
            }
        };
        if blocks.len() > 0 {
            log::info!(
                "Read {} blocks from RwLockBuffer in {:?}",
                blocks.len(),
                now.elapsed()
            );
        }
        blocks
    }
}
