use crate::ethereum::handler::EthereumHandler;
use massbit_chain_ethereum::data_type::ExtBlock;
use graph::prelude::web3::types::{Transaction, TransactionReceipt};
use crate::storage_adapter::StorageAdapter;

pub struct EthereumDailyAddressTransaction {
    pub storage_adapter: Box<dyn StorageAdapter>,
}

impl EthereumDailyAddressTransaction {
    pub fn new(storage_adapter: Box<dyn StorageAdapter>) -> Self {
        EthereumDailyAddressTransaction {
            storage_adapter
        }
    }
}

impl EthereumHandler for EthereumDailyAddressTransaction {
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}