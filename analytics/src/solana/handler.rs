//Public trait for ethereum metric
use massbit_common::prelude::anyhow;
use massbit_chain_solana::data_type::SolanaBlock;
use std::collections::HashMap;
use super::metrics::*;
use std::sync::Arc;
use crate::storage_adapter::StorageAdapter;
use massbit_common::NetworkType;

pub trait SolanaHandler : Sync + Send {
    fn handle_block(&self, _block: &SolanaBlock) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn handle_blocks(&self, _vec_blocks: &Vec<SolanaBlock>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[derive(Default)]
pub struct SolanaHandlerManager {
    pub handlers: Vec<Box<dyn SolanaHandler>>
}
impl SolanaHandlerManager {
    pub fn new() -> SolanaHandlerManager {
        SolanaHandlerManager::default()
    }
    pub fn add_handler(mut self, handler: Box<dyn SolanaHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
    pub fn handle_ext_block(&self, block: &SolanaBlock) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            handler.handle_block(block);
        });
        Ok(())
    }
}

pub fn create_solana_handler_manager(network: &Option<NetworkType>, storate_adapter: Arc<dyn StorageAdapter>)
    -> SolanaHandlerManager {
    let mut handler_manager = SolanaHandlerManager::new();
    handler_manager
        //.add_handler(Box::new(SolanaRawBlockHandler::new(network, storate_adapter.clone())))
        .add_handler(Box::new(SolanaRawTransactionHandler::new(network, storate_adapter.clone())))
        //.add_handler(Box::new(SolanaDailyTransactionHandler::new(network, storate_adapter.clone())))
        //.add_handler(Box::new(SolanaDailyAddressTransactionHandler::new(network, storate_adapter.clone())))

}