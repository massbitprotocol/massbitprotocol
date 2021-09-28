//Public trait for ethereum metric
use massbit_common::prelude::anyhow;
use massbit_chain_ethereum::data_type::{ExtBlock, LightEthereumBlock};
use graph::prelude::web3::types::{Transaction, TransactionReceipt, H256};
use std::collections::HashMap;
use super::metrics::*;
use std::sync::Arc;
use crate::storage_adapter::StorageAdapter;
use massbit_common::NetworkType;

pub trait EthereumHandler : Sync + Send {
    fn handle_block(&self, vec_blocks: &LightEthereumBlock) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn handle_blocks(&self, vec_blocks: &Vec<LightEthereumBlock>) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn handle_transaction(&self, transaction: &Transaction) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn handle_receipt(&self, receipt: &TransactionReceipt) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn handle_receipts(&self, receipts: &HashMap<H256, TransactionReceipt>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[derive(Default)]
pub struct EthereumHandlerManager {
    pub handlers: Vec<Box<dyn EthereumHandler>>
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
        self.handle_block(&block.block);
        self.handle_transactions(&block.block.transactions);
        self.handle_receipts(&block.receipts);
        Ok(())
    }
}

impl EthereumHandler for EthereumHandlerManager {
    fn handle_block(&self, block: &LightEthereumBlock) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            handler.handle_block(block);
        });
        Ok(())
    }
    fn handle_blocks(&self, vec_blocks: &Vec<LightEthereumBlock>) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            handler.handle_blocks(vec_blocks);
        });
        Ok(())
    }
    fn handle_transaction(&self, transaction: &Transaction) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            handler.handle_transaction(transaction);
        });
        Ok(())
    }
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            handler.handle_transactions(transactions);
        });
        Ok(())
    }
    fn handle_receipt(&self, receipt: &TransactionReceipt) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            handler.handle_receipt(receipt);
        });
        Ok(())
    }
    fn handle_receipts(&self, receipts: &HashMap<H256, TransactionReceipt>) -> Result<(), anyhow::Error> {
        self.handlers.iter().for_each(|handler| {
            handler.handle_receipts(receipts);
        });
        Ok(())
    }
}

pub fn create_ethereum_handler_manager(network: &Option<NetworkType>, storate_adapter: Arc<dyn StorageAdapter>) -> EthereumHandlerManager {
    let mut handler_manager = EthereumHandlerManager::new();
    handler_manager.add_handler(Box::new(EthereumDailyTransaction::new(network, storate_adapter.clone())))
        .add_handler(Box::new(EthereumDailyAddressTransaction::new(network, storate_adapter.clone())))
}