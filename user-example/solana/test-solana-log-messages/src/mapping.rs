use crate::models::SolanaLogMessages;
use massbit_chain_solana::data_type::{
    SolanaBlock,
    SolanaTransaction,
    SolanaLogMessages
};
use uuid::Uuid;
pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_transaction(transaction: &SolanaTransaction) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_log_messages(log_messages: &SolanaLogMessages) -> Result<(), Box<dyn std::error::Error>> {
    //println!("[SO File] Received Solana Log Messages");
    let id = Uuid::new_v4().to_simple().to_string();
    let log_messages_solana_ts = SolanaLogMessages {
        id,
        block_number: log_messages.block_number as i64,
        log_messages: format!("{:?}", log_messages.log_messages),
        signature: format!("{:?}", log_messages.transaction.transaction.signatures),
    };
    log_messages_solana_ts.save();
    Ok(())
}
