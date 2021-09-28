use crate::ethereum::handler::EthereumHandler;
use massbit_chain_ethereum::data_type::ExtBlock;
use graph::prelude::web3::types::{Transaction, TransactionReceipt};
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use massbit_common::NetworkType;

pub struct EthereumDailyAddressTransaction {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumDailyAddressTransaction {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumDailyAddressTransaction {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl EthereumHandler for EthereumDailyAddressTransaction {
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {

        Ok(())
    }
}