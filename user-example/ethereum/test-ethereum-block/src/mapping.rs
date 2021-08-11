use crate::models::*;
use massbit_chain_ethereum::data_type::EthereumBlock;
use uuid::Uuid;

pub fn handle_block(block: &EthereumBlock) -> Result<(), Box<dyn std::error::Error>> {
    //println!("[SO File] Received Ethereum Block");
    let id = Uuid::new_v4().to_simple().to_string();
    let block = Block {
        id,
        block_hash: block.block.hash.clone().unwrap().to_string(),
        block_height: block.block.number.clone().unwrap().as_u64() as i64,
        timestamp: block.block.timestamp.as_u128().to_string(),
    };
    block.save();
    Ok(())
}
