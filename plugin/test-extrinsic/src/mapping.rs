use crate::models::ExtrinsicTs;
use massbit_chain_substrate::data_type::SubstrateUncheckedExtrinsic;

pub fn handle_extrinsic(extrinsic: &SubstrateUncheckedExtrinsic) -> Result<(), Box<dyn std::error::Error>> {
    let string_extrinsic = format!("{:?}", extrinsic.extrinsic);
    let extrinsic_ts = ExtrinsicTs {
        block_number: extrinsic.block_number as i64,
        extrinsic: string_extrinsic,
    };
    extrinsic_ts.save();
    println!("[SO File] Block Number {}", extrinsic.block_number);
    Ok(())
}
