//Public trait for ethereum metric
use super::metrics::*;
use crate::storage_adapter::StorageAdapter;
use graph::prelude::web3::types::{Transaction, TransactionReceipt, H256};
use massbit_chain_ethereum::data_type::{ExtBlock, LightEthereumBlock};
use massbit_common::prelude::anyhow;
use massbit_common::prelude::tokio;
use massbit_common::NetworkType;
use std::collections::HashMap;
use std::sync::Arc;

pub trait EthereumHandler: Sync + Send {
    fn handle_block(&self, block: &ExtBlock) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[derive(Default)]
pub struct EthereumHandlerManager {
    pub handlers: Vec<Box<dyn EthereumHandler>>,
}
impl EthereumHandlerManager {
    pub fn new() -> EthereumHandlerManager {
        EthereumHandlerManager::default()
    }
    pub fn add_handler(mut self, handler: Box<dyn EthereumHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
    pub fn handle_ext_block(&self, block: &ExtBlock) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            //Todo: move each handler to separated thread
            // tokio::spawn( async {
            //     // Process each socket concurrently.
            // });
            handler.handle_block(block);
        });
        Ok(())
    }
}

pub fn create_ethereum_handler_manager(
    network: &Option<NetworkType>,
    storate_adapter: Arc<dyn StorageAdapter>,
) -> EthereumHandlerManager {
    let mut handler_manager = EthereumHandlerManager::new();
    handler_manager
        .add_handler(Box::new(EthereumRawBlockHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Box::new(EthereumRawTransactionHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Box::new(EthereumDailyTransactionHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Box::new(EthereumDailyAddressTransactionHandler::new(
            network,
            storate_adapter.clone(),
        )))
}
