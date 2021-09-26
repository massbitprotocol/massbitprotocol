use crate::ethereum::handler::EthereumHandler;
use massbit_chain_ethereum::data_type::ExtBlock;
use massbit_common::prelude::tokio_postgres::Transaction;
use graph::prelude::web3::types::TransactionReceipt;
use crate::storage_adapter::StorageAdapter;

pub struct EthereumDailyTransaction {

}
impl EthereumDailyTransaction {
    pub fn new(Box<dyn StorageAdapter>) -> Self {

    }
}
impl EthereumHandler for EthereumDailyTransaction {
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {
        Ok(());
    }
}