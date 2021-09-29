use crate::ethereum::handler::EthereumHandler;
use massbit_chain_ethereum::data_type::ExtBlock;
use graph::prelude::web3::types::{Transaction, TransactionReceipt};
use graph::prelude::{Entity, Value, BigInt, BigDecimal as BigDecimalValue, Attribute, chrono};
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use std::collections::HashMap;
use massbit_common::NetworkType;
use index_store::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
use massbit_drive::{FromEntity, ToMap};
use bigdecimal::BigDecimal;
use massbit_common::prelude::bigdecimal::FromPrimitive;
use crate::util::timestamp_round_to_date;
use std::time::{Duration, UNIX_EPOCH};
use crate::postgres_queries::UpsertConflictFragment;
use crate::relational::{Table, Column, ColumnType};
use graph::prelude::chrono::Utc;
use crate::create_entity;
pub struct EthereumDailyAddressTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumDailyAddressTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumDailyAddressTransactionHandler {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl EthereumHandler for EthereumDailyAddressTransactionHandler {
    fn handle_transactions(&self, transactions: &Vec<Transaction>) -> Result<(), anyhow::Error> {
        let values = transactions.iter().map(|tran| {
            //DailyAddressTransactionModel::from(tran).into()
            create_entity_from_transaciton(tran)
        }).collect::<Vec<Entity>>();
        let table = Table::new("ethereum_daily_address_transaction", Some("t"));
        let columns = vec![
            Column::new("address", ColumnType::String),
            Column::new("transaction_date", ColumnType::Varchar),
            Column::new("transaction_count", ColumnType::BigInt),
            Column::new("transaction_volume", ColumnType::BigDecimal),
            Column::new("gas", ColumnType::BigInt)
        ];
        let mut conflict_frag = UpsertConflictFragment::new("ethereum_daily_address_transaction_date_uindex");
        conflict_frag.add_expression("transaction_count", "t.transaction_count + EXCLUDED.transaction_count")
            .add_expression("transaction_volume","t.transaction_volume + EXCLUDED.transaction_volume")
            .add_expression("gas","t.gas + EXCLUDED.gas");
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &values,
                                    Some(conflict_frag));
        Ok(())
    }
}
fn create_entity_from_transaciton(transaction: &Transaction) -> Entity {
    let time = UNIX_EPOCH + Duration::from_secs(block.timestamp.as_u64());
    // Create DateTime from SystemTime
    let datetime = chrono::DateTime::<Utc>::from(time).format("%Y-%m-%d").to_string();
    create_entity!(
        "address" => transaction.from,
        "transaction_date" => datetime
    );
}
// #[derive(Default, Debug, Clone, FromEntity, ToMap)]
// pub struct DailyAddressTransactionModel {
//     pub address: String,
//     pub transaction_date: String,
//     pub timestamp: i64,
//     pub transaction_count: i64,
//     pub transaction_volume: BigDecimalValue,
//     pub gas: BigDecimalValue
// }
//
// impl Into<Entity> for DailyAddressTransactionModel {
//     fn into(self) -> Entity {
//         let map = DailyAddressTransactionModel::to_map(self.clone());
//         Entity::from(map)
//     }
// }
//
// impl From<&Transaction> for DailyAddressTransactionModel {
//     fn from(transaction: &Transaction) -> Self {
//         let parent_hash = format!("{:x}", transaction.parent_hash);
//         let hash = format!("{:x}", transaction.hash.unwrap());
//         // let transaction_hash = format!(
//         //     "0x{}",
//         //     hex::encode(trans.hash.as_bytes()).trim_start_matches('0')
//         // );
//         // let block_number = match transaction.block_number {
//         //     Some(val) => Some(val.as_u64() as i64),
//         //     _ => None
//         // };
//         // let receiver = match transaction.to {
//         //     Some(val) => Some(format!("{:x}",val)),
//         //     _ => None
//         // };
//         let sender = format!("{:x}", transaction.from);
//
//         let value = match BigDecimal::from_u128(transaction.value.as_u128()) {
//             None => BigDecimal::from(0),
//             Some(val) => val
//         };
//         let gas = match BigDecimal::from_u128(transaction.gas.as_u128()) {
//             None => BigDecimal::from(0),
//             Some(val) => val
//         };
//         let gas_price = match BigDecimal::from_u128(transaction.gas_price.as_u128()) {
//             None => BigDecimal::from(0),
//             Some(val) => val
//         };
//         let timestamp = timestamp_round_to_date(block.timestamp.as_u64()) as i64;
//         let time = UNIX_EPOCH + Duration::from_secs(block.timestamp.as_u64());
//         // Create DateTime from SystemTime
//         let datetime = chrono::DateTime::<Utc>::from(time);
//         let date = datetime.format("%Y-%m-%d").to_string();
//         DailyAddressTransactionModel {
//             address: sender,
//             transaction_date: date,
//             timestamp,
//             transaction_count: 1,
//             transaction_volume: BigDecimalValue::from(value),
//             gas: BigDecimalValue::from(gas)
//         }
//     }
// }