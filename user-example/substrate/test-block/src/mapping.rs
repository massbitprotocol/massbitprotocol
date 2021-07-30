use crate::models::*;
use massbit_chain_substrate::data_type as substrate_types;
use uuid::Uuid;

pub fn handle_block(block: &substrate_types::SubstrateBlock) -> Result<(), Box<dyn std::error::Error>> {
    println!("[SO File] Received Substrate Block");
    let block_id = Uuid::new_v4().to_simple().to_string();
    let block_ts = SubstrateBlock {
        id: block_id.clone(),
        block_hash: block.block.header.hash().to_string(),
        block_height: block.block.header.number as i64,
    };
    block_ts.save();
    Ok(())
}
