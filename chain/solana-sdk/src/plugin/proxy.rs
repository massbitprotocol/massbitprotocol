use crate::plugin::{handler::SolanaHandler, BlockResponse, MessageHandler};
use crate::store::IndexStore;
use crate::types::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use crate::COMPONENT_NAME;
use libloading::Library;
use std::error::Error;
use std::sync::Arc;

/// A proxy object which wraps a [`Handler`] and makes sure it can't outlive
/// the library it came from.
pub struct SolanaHandlerProxy {
    pub handler: Box<dyn SolanaHandler + Send + Sync>,
}
impl SolanaHandlerProxy {
    pub fn new(handler: Box<dyn SolanaHandler + Send + Sync>) -> SolanaHandlerProxy {
        SolanaHandlerProxy { handler }
    }
}
impl SolanaHandler for SolanaHandlerProxy {
    fn handle_block(&self, message: &SolanaBlock) -> Result<(), Box<dyn Error>> {
        self.handler.handle_block(message)
    }
    fn handle_transaction(&self, message: &SolanaTransaction) -> Result<(), Box<dyn Error>> {
        self.handler.handle_transaction(message)
    }
    fn handle_log_messages(&self, message: &SolanaLogMessages) -> Result<(), Box<dyn Error>> {
        self.handler.handle_log_messages(message)
    }
}

impl MessageHandler for SolanaHandlerProxy {
    fn handle_block_mapping(
        &self,
        data: &mut BlockResponse,
        store: &mut dyn IndexStore,
    ) -> Result<i64, Box<dyn Error>> {
        //log::info!("handle_block_mapping data: {:?}", data);
        let mut block_slot = -1_i64;
        let blocks: Vec<SolanaBlock> = serde_json::from_slice(&mut data.payload).unwrap();
        // Todo: Rewrite the flush so it will flush after finish the array of blocks for better performance. For now, we flush after each block.
        for block in blocks {
            log::info!(
                "{} Received SOLANA BLOCK with block slot: {:?} and hash {:?}, with {} TRANSACTIONs",
                &*COMPONENT_NAME,
                &block.block_number,
                &block.block.blockhash,
                &block.block.transactions.len()
            );
            self.handler.handle_block(&block);
            store.flush(&block.block.blockhash, block.block_number);
            block_slot = block_slot.max(block.block_number as i64);
        }
        Ok(block_slot)
    }
}
