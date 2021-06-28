use super::models::{Block, Extrinsic};
use plugin_core::{BlockHandler, ExtrinsicHandler, InvocationError};
use types::{SubstrateBlock, SubstrateExtrinsic};

#[derive(Debug, Clone, PartialEq)]
pub struct BlockIndexer;

impl BlockHandler for BlockIndexer {
    fn handle_block(&self, substrate_block: &SubstrateBlock) -> Result<(), InvocationError> {
        let block = Block {
            id: substrate_block.idx,
        };
        block.save();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtrinsicIndexer;

impl ExtrinsicHandler for ExtrinsicIndexer {
    fn handle_extrinsic(
        &self,
        substrate_extrinsic: &SubstrateExtrinsic,
    ) -> Result<(), InvocationError> {
        let extrinsic = Extrinsic {
            id: substrate_extrinsic.idx,
        };
        extrinsic.save();
        Ok(())
    }
}
