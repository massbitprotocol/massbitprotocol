//Public trait for ethereum metric
use massbit_common::prelude::anyhow;
use massbit_chain_ethereum::data_type::ExtBlock;
use graph::prelude::web3::types::{Transaction, TransactionReceipt};

pub trait EthereumHandler : Sync + Send {
    fn handle_block(&self, block: &ExtBlock) -> Result<(), anyhow::Error> {
        Ok(());
    }
    fn handle_blocks(&self, vec_blocks: &Vec<ExtBlock>) -> Result<(), anyhow::Error> {
        Ok(());
    }
    fn handle_transaction(&self, transaction: &Transaction) -> Result<(), anyhow::Error> {
        Ok(());
    }
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {
        Ok(());
    }
    fn handle_receipt(&self, receipt: &TransactionReceipt) -> Result<(), anyhow::Error> {
        Ok(());
    }
    fn handle_receipts(&self, receipts: &Vec<TransactionReceipt>) -> Result<(), anyhow::Error> {
        Ok(());
    }
}
