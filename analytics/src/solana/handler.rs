//Public trait for ethereum metric
use super::metrics::*;
use crate::storage_adapter::StorageAdapter;
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::prelude::anyhow;
use massbit_common::NetworkType;
use solana_transaction_status::{ConfirmedBlock, EncodedConfirmedBlock};
use std::sync::Arc;

pub trait SolanaHandler: Sync + Send {
    fn handle_block(
        &self,
        block_slot: u64,
        _block: Arc<EncodedConfirmedBlock>,
    ) -> Result<(), anyhow::Error>;
    //fn handle_blocks(&self, _blocks: Arc<Vec<EncodedConfirmedBlock>>) -> Result<(), anyhow::Error>;
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
    pub fn handle_block(
        &self,
        block_slot: u64,
        block: Arc<EncodedConfirmedBlock>,
    ) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            let clone_handler = handler.clone();
            let clone_block = Arc::clone(&block);
            tokio::spawn(async move {
                match clone_handler.handle_block(block_slot, clone_block) {
                    Ok(_) => {}
                    Err(err) => log::error!("{:?}", &err),
                }
            });
        });
        Ok(())
    }
}

pub fn create_solana_handler_manager(
    network: &Option<NetworkType>,
    storate_adapter: Arc<dyn StorageAdapter>,
) -> SolanaHandlerManager {
    let handler_manager = SolanaHandlerManager::new();
    handler_manager
        .add_handler(Arc::new(SolanaRawBlockHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(SolanaRawTransactionHandler::new(
            network,
            storate_adapter.clone(),
        )))
        // .add_handler(Arc::new(SolanaRawLogHandler::new(
        //     network,
        //     storate_adapter.clone(),
        // )))
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
}
