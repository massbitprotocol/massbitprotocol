use crate::models::BlockTs;

use massbit_chain_substrate::data_type::SubstrateBlock;
use plugin::core::BlockHandler;
use store::Store;

#[derive(Debug, Clone, PartialEq)]
pub struct Indexer;

impl BlockHandler for Indexer {
    fn handle_block(
        &self,
        store: &mut dyn Store,
        block: &SubstrateBlock,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let block_ts = BlockTs {
            block_hash: block.header.hash().to_string(),
            block_height: block.header.number as i64,
        };
        store.save("blocks".to_string(), block_ts.into());
        Ok(())
    }
}
