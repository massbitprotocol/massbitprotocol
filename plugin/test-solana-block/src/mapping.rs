use crate::models::BlockSolanaTs;
use massbit_chain_solana::data_type::{
    SolanaBlock,
    SolanaTransaction,
    SolanaLogMessages
};

pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    println!("[SO File] Received Solana Block");
    let block_solana_ts = BlockSolanaTs {
        block_hash: block.block.blockhash.clone(),
        block_height: block.block.block_height.unwrap() as i64,
        timestamp: block.block.block_time.unwrap().to_string(),
    };
    block_solana_ts.save();
    Ok(())
}

pub fn handle_transaction(transaction: &SolanaTransaction) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_log_messages(event: &SolanaLogMessages) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
