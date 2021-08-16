use crate::models::*;
use massbit_chain_substrate::data_type as substrate_types;
use uuid::Uuid;

pub fn handle_extrinsic(
    extrinsic: &substrate_types::SubstrateUncheckedExtrinsic,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = Uuid::new_v4().to_simple().to_string();
    let string_extrinsic = format!("{:?}", extrinsic.extrinsic);
    let extrinsic_ts = SubstrateExtrinsic {
        id: id.clone(),
        block_number: extrinsic.block_number as i64,
        extrinsic: string_extrinsic,
    };
    extrinsic_ts.save();
    Ok(())
}
