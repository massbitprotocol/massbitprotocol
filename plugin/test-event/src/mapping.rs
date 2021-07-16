use crate::models::EventTs;
use massbit_chain_substrate::data_type::{SubstrateEventRecord, SubstrateUncheckedExtrinsic, SubstrateBlock};
use chrono;

pub fn handle_event(event: &SubstrateEventRecord) -> Result<(), Box<dyn std::error::Error>> {
    println!("[SO File] Received Event");
    let event_ts = EventTs {
        event: format!("{:?}", event.event),
        timestamp: format!("{:?}", chrono::offset::Utc::now()),
    };
    event_ts.save();
    Ok(())
}

pub fn handle_extrinsic(extrinsic: &SubstrateUncheckedExtrinsic) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn handle_block(block: &SubstrateBlock) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}


