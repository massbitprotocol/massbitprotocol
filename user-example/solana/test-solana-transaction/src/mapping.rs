use crate::models::Transaction;
use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use uuid::Uuid;
pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_transaction(
    transaction: &SolanaTransaction,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create ID
    let transaction_id = Uuid::new_v4().to_simple().to_string();
    let transaction_solana_ts = SolanaTransactionTs {
        id: transaction_id,
        block_number: transaction.block_number as i64,
        fee: transaction.transaction.meta.clone().unwrap().fee as i64,
        signature: format!("{:?}", transaction.transaction.transaction.signatures),
    };
    transaction_solana_ts.save();
    Ok(())
}
