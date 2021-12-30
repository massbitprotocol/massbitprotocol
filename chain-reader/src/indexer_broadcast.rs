use chain_solana::types::{BlockInfo, ConfirmedBlockWithSlot};
use log::{debug, info};
use massbit::prelude::Future;
use massbit::slog::log;
use massbit_chain_solana::data_type::{ExtBlock, SolanaBlock, SolanaFilter};
use massbit_grpc::firehose::bstream::BlockResponse;
use solana_sdk::slot_history::Slot;
use solana_transaction_status::ConfirmedBlock;
use std::collections::{HashMap, HashSet, VecDeque};
use std::iter::FromIterator;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task;
use tonic::Status;

const VERSION: &str = "1.7.0";
const MAX_BUFFER_SIZE: usize = 1000_usize;
#[derive(Default)]
pub struct BlockBuffer {
    /// Map parent_slot => ConfirmedBlock
    buffer: HashMap<u64, ConfirmedBlockWithSlot>,
    /// Expected slots
    expected_slots: VecDeque<u64>,
}

impl BlockBuffer {
    fn handle_incoming_block(&mut self, block_info: BlockInfo) -> Vec<ConfirmedBlockWithSlot> {
        let mut blocks = vec![];
        match block_info {
            BlockInfo::BlockSlots(slots) => {
                //Current block_slot
                debug!("*** handle_incoming_block receive block: {:?}", &slots);
                for slot in slots.iter() {
                    self.expected_slots.push_back(slot.clone());
                }
                //If buffer is full then send first block to indexers
                while self.expected_slots.len() >= MAX_BUFFER_SIZE {
                    if let Some(slot) = self.expected_slots.pop_front() {
                        if let Some(block) = self.buffer.remove(&slot) {
                            blocks.push(block);
                        }
                    }
                }
                //Get all exists blocks in queues
                loop {
                    let first_slot = self.expected_slots.get(0).unwrap().clone();
                    if let Some(block) = self.buffer.remove(&first_slot) {
                        blocks.push(block);
                        self.expected_slots.pop_front();
                    } else {
                        break;
                    }
                }
            }
            BlockInfo::ConfirmBlockWithSlot(confirm_block) => {
                debug!("*** Receive block: {}", &confirm_block.block_slot);
                if self.expected_slots.len() == 0 {
                    self.buffer.insert(confirm_block.block_slot, confirm_block);
                } else {
                    let first_slot = self.expected_slots.get(0).unwrap().clone();
                    if first_slot == confirm_block.block_slot {
                        blocks.push(confirm_block);
                        loop {
                            // Remove received block
                            self.expected_slots.pop_front();
                            if let Some(next_block) = self.expected_slots.get(0) {
                                if let Some(block) = self.buffer.remove(next_block) {
                                    blocks.push(block);
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    } else if first_slot < confirm_block.block_slot {
                        self.buffer.insert(confirm_block.block_slot, confirm_block);
                    } else {
                        log::warn!(
                            "Block {:?} come too late after remove expected slot",
                            confirm_block.block_slot
                        );
                    }
                }
            }
        }
        blocks
    }
}
pub struct IndexerInfo {
    hash: String, //Indexer hash
    filter: SolanaFilter,
    filter_hashes: HashSet<String>, //For quickly filter ConfirmedBlock
    sender: Sender<Result<BlockResponse, Status>>,
}
pub struct IndexerBroadcast {
    block_receiver: Receiver<BlockInfo>,
    block_buffer: BlockBuffer,
    ind_senders: Mutex<Vec<IndexerInfo>>,
}

impl IndexerBroadcast {
    pub fn new(receiver: Receiver<BlockInfo>) -> Self {
        IndexerBroadcast {
            block_receiver: receiver,
            block_buffer: BlockBuffer::default(),
            ind_senders: Mutex::new(vec![]),
        }
    }
    ///Init broadcast thread
    pub async fn try_recv(&mut self) -> bool {
        match self.block_receiver.try_recv() {
            Ok(data) => {
                let blocks = self.block_buffer.handle_incoming_block(data);
                if blocks.len() > 0 {
                    log::info!(
                        "Broadcast blocks: {:?}",
                        blocks
                            .iter()
                            .map(|block| block.block_slot)
                            .collect::<Vec<Slot>>()
                    );
                    self.broadcast_blocks(blocks).await;
                }
                return true;
            }
            Err(e) => {
                debug!("try_recv error: {:?}", e);
                return false;
            }
        }
    }
    ///Call from main thread to add new indexer
    pub fn register_indexer(
        &mut self,
        hash: &String,
        encoded_filter: &Vec<u8>,
        indexer_sender: Sender<Result<BlockResponse, Status>>,
    ) {
        // Decode filter
        let filter: SolanaFilter = serde_json::from_slice(&encoded_filter).unwrap_or_default();
        let mut filter_hashes = HashSet::default();
        filter.keys.iter().for_each(|key| {
            filter_hashes.insert(key.to_string());
        });
        ///Create block buffer to store received block from ChainDispatcher
        let mut senders = self.ind_senders.lock().unwrap();
        senders.push(IndexerInfo {
            hash: hash.clone(),
            filter,
            filter_hashes,
            sender: indexer_sender,
        });
    }
    async fn broadcast_blocks(&mut self, block_with_slots: Vec<ConfirmedBlockWithSlot>) {
        debug!("*** broadcast_blocks");
        let mut filtered_blocks: HashMap<String, Vec<ConfirmedBlockWithSlot>> = HashMap::default();
        let mut indexers = self.ind_senders.lock().unwrap();
        //Remove stop indexers
        indexers.retain(|indexer| !indexer.sender.is_closed());
        block_with_slots
            .iter()
            .filter(|block| block.block.is_some())
            .for_each(|block| {
                let ref_block = block.block.as_ref().unwrap();
                //Clone ConfirmedBlock for each indexer with empty transactions
                let mut indexer_blocks: HashMap<_, _> = HashMap::from_iter(
                    indexers
                        .iter()
                        .map(|indexer| (indexer.hash.clone(), block.cheap_clone())),
                );
                //Iterate throw transactions and clone it for interested indexer
                ref_block.transactions.iter().for_each(|tran| {
                    let keys = tran
                        .transaction
                        .message
                        .account_keys
                        .iter()
                        .map(|key| key.to_string())
                        .collect::<Vec<String>>();
                    indexers.iter().for_each(|indexer| {
                        if keys.iter().any(|key| indexer.filter_hashes.contains(key)) {
                            indexer_blocks
                                .get_mut(&indexer.hash)
                                .unwrap()
                                .block
                                .as_mut()
                                .unwrap()
                                .transactions
                                .push(tran.clone());
                        }
                    });
                });
                indexer_blocks.into_iter().for_each(|(hash, block)| {
                    if block.block.as_ref().unwrap().transactions.len() > 0 {
                        filtered_blocks
                            .entry(hash)
                            .or_insert_with(Vec::new)
                            .push(block);
                    }
                });
            });
        for indexer in indexers.iter() {
            if let Some(blocks) = filtered_blocks.remove(&indexer.hash) {
                let block_response = Self::create_block_response(blocks);
                debug!("*** GRPC Send block_response");
                indexer.sender.send(Ok(block_response)).await;
            }
        }
        // indexers.iter().for_each(|indexer| {
        //     if let Some(blocks) = filtered_blocks.remove(&indexer.hash) {
        //         let block_response = Self::create_block_response(blocks);
        //         debug!("*** GRPC Send block_response");
        //         indexer.sender.send(Ok(block_response)).await;
        //     }
        // });
    }
    fn create_block_response(blocks: Vec<ConfirmedBlockWithSlot>) -> BlockResponse {
        let ext_blocks = blocks
            .into_iter()
            .map(|block_with_slot| {
                let ConfirmedBlockWithSlot { block_slot, block } = block_with_slot;
                debug!("Add block: {}", &block_slot);
                let ref_block = block.as_ref().unwrap();
                let timestamp = ref_block.block_time.unwrap_or_default();
                let list_log_messages = ref_block
                    .transactions
                    .iter()
                    .map(|transaction| transaction.meta.as_ref().unwrap().log_messages.clone())
                    .collect();
                ExtBlock {
                    version: VERSION.to_string(),
                    timestamp,
                    block_number: block_slot,
                    block: block.unwrap(),
                    list_log_messages,
                }
            })
            .collect::<Vec<ExtBlock>>();
        BlockResponse {
            version: VERSION.to_string(),
            payload: serde_json::to_vec(&ext_blocks).unwrap(),
        }
    }
}
