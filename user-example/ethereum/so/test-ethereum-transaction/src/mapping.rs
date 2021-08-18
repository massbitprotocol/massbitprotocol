use crate::models::*;
use massbit_chain_ethereum::data_type::EthereumTransaction;
use uuid::Uuid;

pub fn handle_transaction(
    transaction: &EthereumTransaction,
) -> Result<(), Box<dyn std::error::Error>> {
    //println!("[SO File] Received Ethereum Block");
    let id = Uuid::new_v4().to_simple().to_string();
    let transaction = EthereumTransactionTable {
        id,
        block_number: transaction
            .transaction
            .block_number
            .clone()
            .unwrap()
            .as_u64() as i64,
        timestamp: transaction.timestamp.to_string(),
        transaction_hash: transaction.transaction.hash.to_string(),
        receipt: format!("{:?}", transaction.receipt),
    };
    transaction.save();
    Ok(())
}
