use crate::models::*;
use massbit_chain_ethereum::data_type::EthereumEvent;
use uuid::Uuid;

pub fn handle_event(event: &EthereumEvent) -> Result<(), Box<dyn std::error::Error>> {
    //println!("[SO File] Received Ethereum Event");
    for log in event.logs.clone() {
        let id = Uuid::new_v4().to_simple().to_string();
        let event = EthereumEventTable {
            id: id.clone(),
            block_number: log.block_number.clone().unwrap().as_u64() as i64,
            address: log.address.to_string(),
            log_type: format!("{:?}", log.log_type),
            transaction_hash: format!("{:?}", log.transaction_hash),
        };
        event.save();
    }
    Ok(())
}
