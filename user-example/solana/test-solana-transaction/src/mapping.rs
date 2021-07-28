use crate::models::TransactionSolanaTs;
use massbit_chain_solana::data_type::{
    SolanaBlock,
    SolanaTransaction,
    SolanaLogMessages
};

pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_transaction(transaction: &SolanaTransaction) -> Result<(), Box<dyn std::error::Error>> {
    println!("[SO File] Received Solana Transaction");

    let transaction_solana_ts = TransactionSolanaTs {
        block_number: transaction.block_number as i64,
        fee: transaction.transaction.meta.clone().unwrap().fee as i64,
        signature: format!("{:?}", transaction.transaction.transaction.signatures),
    };
    transaction_solana_ts.save();
    Ok(())
}

pub fn handle_log_messages(event: &SolanaLogMessages) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
