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
    //println!("[SO File] Received Ethereum Block");
    let id = format!(
        "0x{}",
        hex::encode(transaction.transaction.hash.as_bytes()).trim_start_matches('0')
    );
    let sender = format!(
        "0x{}",
        hex::encode(transaction.transaction.from.as_bytes()).trim_start_matches('0')
    );
    let timestamp: u64 = transaction.timestamp.into();
    let d = UNIX_EPOCH + Duration::from_secs(timestamp);
    // Create DateTime from SystemTime
    let datetime = DateTime::<Utc>::from(d);
    // Formats the combined date and time with the specified format string.
    //let timestamp_str = datetime.format("%Y-%m-%d %H:%M:%S.%f").to_string();
    let date = datetime.format("%Y-%m-%d").to_string();
    format!("{}", date);
    // let stored_value = MaticTransactionTable::get(
    //     "0x880c0ebfd3873d74f7088f1025d5c0b4763e8934df75b63f9032a6e4dff04160".to_string(),
    // );
    // println!("{:?}", stored_value);
    //Test query
    // let entity_filter = EntityFilter::And(vec![
    //     EntityFilter::Equal(
    //         "sender".to_string(),
    //         Value::from("0x7bf377f69da0e46da1502d5f2bcf9fb00c3b610b"),
    //     ),
    //     EntityFilter::Equal("date".to_string(), Value::from("2021-08-26")),
    // ]);
    // let vec = MaticTransactionTable::query(
    //     Some(entity_filter),
    //     EntityOrder::Default,
    //     EntityRange::first(10),
    // );
    // println!("{:?}", &vec);
    let mut daily_active = DailyActiveAddress::get(&sender);
    match daily_active {
        None => {
            let mut daily_active = DailyActiveAddress {
                id: sender.clone(),
                chain: "Polygon".to_string(),
                counter: 1,
            };
            daily_active.save();
        }
        Some(mut daily_active) => {
            daily_active.counter = daily_active.counter + 1;
            daily_active.save();
        }
    }
    let transaction = MaticTransactionTable {
        id,
        sender,
        date,
        timestamp: timestamp as i64,
    };
    transaction.save();

    Ok(())
}
