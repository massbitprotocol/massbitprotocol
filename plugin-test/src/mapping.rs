use crate::models::BlockTs;

use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::BlockHandler;

#[derive(Debug, Clone, PartialEq)]
pub struct Indexer;

impl BlockHandler for Indexer {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn std::error::Error>> {
        let block_ts = BlockTs {
            block_hash: block.header.hash().to_string(),
            block_height: block.header.number as i64,
        };
        block_ts.save();
        Ok(())
    }
}
