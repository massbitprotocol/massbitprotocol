use crate::models::*;
use massbit_chain_substrate::data_type as substrate_types;
use uuid::Uuid;

pub fn handle_event(
    event: &substrate_types::SubstrateEventRecord,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = Uuid::new_v4().to_simple().to_string();
    let event_ts = SubstrateEvent {
        id: id.clone(),
        event: format!("{:?}", event.event),
        timestamp: format!("{:?}", chrono::offset::Utc::now()),
    };
    event_ts.save();
    Ok(())
}
