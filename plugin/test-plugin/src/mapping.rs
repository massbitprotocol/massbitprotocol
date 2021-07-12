use crate::models::{BlockTs, Extrinsic};
use massbit_chain_substrate::data_type::SubstrateBlock;
use std::error::Error;

pub fn handle_block(block: &SubstrateBlock) -> Result<(), Box<dyn Error>> {
    let block_ts = BlockTs {
        id: block.header.hash().to_string(),
        block_height: block.header.number as i64,
    };
    block_ts.save();

    // for ref extrinsic in block.extrinsics {
    //     if extrinsic.is_signed()? {
    //         let entity = Extrinsic {
    //             id: extrinsic.get_hash().to_string(),
    //             block_hash: block.header.hash().to_string(),
    //             block_height: block.header.number as i64,
    //             origin: extrinsic.signature.to_string(),
    //         };
    //         entity.save()
    //     }
    // }

    Ok(())
}
