use crate::models::BlockTs;
use massbit_chain_substrate::data_type::SubstrateBlock;

pub fn handle_block(block: &SubstrateBlock) -> Result<(), Box<dyn std::error::Error>> {
    let block_ts = BlockTs {
        block_hash: block.block.header.hash().to_string(),
        block_height: block.block.header.number as i64,
    };
    block_ts.save();
    Ok(())
}
