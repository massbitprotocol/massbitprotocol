use crate::models::*;
use massbit_chain_ethereum::data_type::EthereumEvent;
use uuid::Uuid;

pub fn handle_event(event: &EthereumEvent) -> Result<(), Box<dyn std::error::Error>> {
    //println!("[SO File] Received Ethereum Event");
    let id = Uuid::new_v4().to_simple().to_string();
    let event_ts = EthereumEventTable {
        id,
        block_number: event.event.block.number.as_u64() as i64,
        address: event.event.address.to_string(),
        log_type: format!("{:?}", event.event.log_type),
        transaction_hash: format!("{:?}", event.event.transaction.hash),
    };
    event_ts.save();
    Ok(())
}
