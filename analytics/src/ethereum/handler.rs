//Public trait for ethereum metric
use super::metrics::*;
use crate::storage_adapter::StorageAdapter;
use massbit::prelude::LightEthereumBlock;
use massbit_common::prelude::anyhow;
use massbit_common::NetworkType;
use std::sync::Arc;

pub trait EthereumHandler: Sync + Send {
    fn handle_block(&self, _block: Arc<LightEthereumBlock>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[derive(Default)]
pub struct EthereumHandlerManager {
    pub handlers: Vec<Arc<dyn EthereumHandler>>,
}
impl EthereumHandlerManager {
    pub fn new() -> EthereumHandlerManager {
        EthereumHandlerManager::default()
    }
    pub fn add_handler(mut self, handler: Arc<dyn EthereumHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
    pub fn handle_block(&self, block: Arc<LightEthereumBlock>) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            let clone_handler = handler.clone();
            let clone_block = Arc::clone(&block);
            tokio::spawn(async move {
                match clone_handler.handle_block(clone_block) {
                    Ok(_) => {}
                    Err(err) => log::error!("{:?}", &err),
                }
            });
        });
        Ok(())
    }
}

pub fn create_ethereum_handler_manager(
    network: &Option<NetworkType>,
    storate_adapter: Arc<dyn StorageAdapter>,
) -> EthereumHandlerManager {
    let handler_manager = EthereumHandlerManager::new();
    handler_manager
        .add_handler(Arc::new(EthereumRawBlockHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(EthereumRawTransactionHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(EthereumDailyTransactionHandler::new(
            network,
            storate_adapter.clone(),
        )))
        .add_handler(Arc::new(EthereumDailyAddressTransactionHandler::new(
            network,
            storate_adapter.clone(),
        )))
}
