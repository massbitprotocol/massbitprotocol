extern crate chrono;
use crate::models::*;
use chrono::prelude::DateTime;
use chrono::Utc;
use hex;
use massbit_chain_ethereum::data_type::EthereumTransaction;
use std::time::{Duration, UNIX_EPOCH};

pub fn handle_transaction(
    transaction: &EthereumTransaction,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = format!(
        "0x{}",
        hex::encode(transaction.transaction.hash.as_bytes()).trim_start_matches('0')
    );
    let sender = format!(
        "0x{}",
        hex::encode(transaction.transaction.from.as_bytes()).trim_start_matches('0')
    );
    let receiver = match transaction.transaction.to {
        Some(val) => format!(
            "0x{}",
            hex::encode(val.as_bytes()).trim_start_matches('0')
        ),
        _ => "".to_string()
    };
    let timestamp: u64 = transaction.timestamp.into();
    let time = UNIX_EPOCH + Duration::from_secs(timestamp);
    // Create DateTime from SystemTime
    let datetime = DateTime::<Utc>::from(time);
    // Formats the combined date and time with the specified format string.
    //let timestamp_str = datetime.format("%Y-%m-%d %H:%M:%S.%f").to_string();
    let date = datetime.format("%Y-%m-%d").to_string();
    let block_hash = match transaction.transaction.block_hash {
        None => "".to_string(),
        Some(hash) => format!(
            "0x{}",
            hex::encode(hash.as_bytes()).trim_start_matches('0')
        ),
    };
    let block_number = match transaction.transaction.block_number {
        Some(val) => val.as_u64() as i64,
        _ => 0
    };

    let transaction = MaticTransactionTable {
        id,
        block_hash,
        block_number,
        nonce: transaction.transaction.nonce.as_u128() as i64,
        sender,
        receiver,
        value: transaction.transaction.value.as_u128() as i64,
        gas: transaction.transaction.gas.as_u128() as i64,
        gas_price: transaction.transaction.gas_price.as_u128() as i64,
        date,
        timestamp: timestamp as i64,
    };
    transaction.save();

    Ok(())
}
