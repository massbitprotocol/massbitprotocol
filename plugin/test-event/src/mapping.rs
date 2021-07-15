use crate::models::EventTs;
use massbit_chain_substrate::data_type::SubstrateEventRecord;
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
