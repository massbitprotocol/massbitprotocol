use crate::models::LogMessagesSolanaTs;
use massbit_chain_solana::data_type::{
    SolanaBlock,
    SolanaTransaction,
    SolanaLogMessages
};

pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_transaction(transaction: &SolanaTransaction) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_log_messages(log_messages: &SolanaLogMessages) -> Result<(), Box<dyn std::error::Error>> {
    println!("[SO File] Received Solana Log Messages");

    let log_messages_solana_ts = LogMessagesSolanaTs {
        block_number: log_messages.block_number as i64,
        log_messages: format!("{:?}", log_messages.log_messages),
        signature: format!("{:?}", log_messages.transaction.transaction.transaction.signatures),
    };
    log_messages_solana_ts.save();
    Ok(())
}
