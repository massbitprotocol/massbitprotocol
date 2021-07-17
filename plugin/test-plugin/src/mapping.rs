use crate::models::Block;
use massbit_chain_substrate::data_type::SubstrateBlock;
use std::error::Error;

pub fn handle_block(block: &SubstrateBlock) -> Result<(), Box<dyn Error>> {
    let block_ts = Block {
        block_hash: block.block.header.hash().to_string(),
        block_height: block.block.header.number as i64,
    };
    block_ts.save();
    Ok(())
}
