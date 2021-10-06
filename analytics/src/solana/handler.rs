//Public trait for ethereum metric
use super::metrics::*;
use crate::storage_adapter::StorageAdapter;
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::prelude::anyhow;
use massbit_common::NetworkType;
use std::collections::HashMap;
use std::sync::Arc;

pub trait SolanaHandler: Sync + Send {
    fn handle_block(&self, _block: Arc<SolanaBlock>) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn handle_blocks(&self, _vec_blocks: Arc<Vec<SolanaBlock>>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[derive(Default)]
pub struct SolanaHandlerManager {
    pub handlers: Vec<Arc<dyn SolanaHandler>>,
}
impl SolanaHandlerManager {
    pub fn new() -> SolanaHandlerManager {
        SolanaHandlerManager::default()
    }
    pub fn add_handler(mut self, handler: Arc<dyn SolanaHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
    pub fn handle_ext_block(&self, block: Arc<SolanaBlock>) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            let clone_handler = handler.clone();
            let clone_block = Arc::clone(&block);
            tokio::spawn(async move {
                clone_handler.handle_block(clone_block);
            });
        });
        Ok(())
    }
}

pub fn create_solana_handler_manager(
    network: &Option<NetworkType>,
    storate_adapter: Arc<dyn StorageAdapter>,
) -> SolanaHandlerManager {
    let mut handler_manager = SolanaHandlerManager::new();
    handler_manager
        .add_handler(Arc::new(SolanaRawBlockHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(SolanaRawTransactionHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(SolanaRawLogHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(SolanaInstructionHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(SolanaTokenBalanceHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(SolanaStatBlockHandler::new(
            network,
            storate_adapter.clone(),
        )))
    //.add_handler(Box::new(SolanaDailyTransactionHandler::new(network, storate_adapter.clone())))
    //.add_handler(Box::new(SolanaDailyAddressTransactionHandler::new(network, storate_adapter.clone())))
}
