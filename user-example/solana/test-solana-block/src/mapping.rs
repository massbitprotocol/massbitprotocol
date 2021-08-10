use crate::models::*;
use massbit_chain_solana::data_type::{
    SolanaBlock,
    SolanaTransaction,
    SolanaLogMessages
};
use uuid::Uuid;

pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    //println!("[SO File] Received Solana Block");
    let id = Uuid::new_v4().to_simple().to_string();
    let block = Block {
        id,
        block_hash: block.block.blockhash.clone(),
        block_height: block.block.block_height.unwrap() as i64,
        timestamp: block.block.block_time.unwrap().to_string(),
    };
    block.save();
    Ok(())
}
