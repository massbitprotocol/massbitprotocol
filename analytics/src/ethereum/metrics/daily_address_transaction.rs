use crate::ethereum::handler::EthereumHandler;
use massbit_chain_ethereum::data_type::ExtBlock;
use massbit_common::prelude::tokio_postgres::Transaction;
use graph::prelude::web3::types::TransactionReceipt;

pub struct EthereumDailyAddressTransaction {

}

impl EthereumDailyAddressTransaction {
    pub fn new(Box<dyn StorageAdapter>) -> Self {

    }
}

impl EthereumHandler for EthereumDailyAddressTransaction {
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {
        Ok(());
    }
}