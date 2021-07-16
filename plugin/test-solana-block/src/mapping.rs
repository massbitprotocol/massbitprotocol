use crate::models::BlockSolanaTs;
use massbit_chain_solana::data_type::{
    SolanaBlock,
    SolanaTransaction,
    SolanaEvent,
};

pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    println!("[SO File] Received Block");
    Ok(())
}

pub fn handle_transaction(transaction: &SolanaTransaction) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_event(event: &SolanaEvent) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
