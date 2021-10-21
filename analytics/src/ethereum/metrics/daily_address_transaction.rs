use crate::ethereum::handler::EthereumHandler;
use crate::postgres_queries::UpsertConflictFragment;
use crate::relational::{Column, ColumnType, Table};
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use chrono::{self, Utc};
use core::ops::Add;
use massbit::components::ethereum::LightEthereumBlock;
use massbit::prelude::{Attribute, BigInt, Entity, Value};
use massbit_common::NetworkType;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

pub struct EthereumDailyAddressTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumDailyAddressTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumDailyAddressTransactionHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl EthereumHandler for EthereumDailyAddressTransactionHandler {
    fn handle_block(&self, block: Arc<LightEthereumBlock>) -> Result<(), anyhow::Error> {
        let values = create_entities(block.as_ref());
        let table = Table::new("ethereum_daily_address_transactions", create_columns());
        let mut conflict_frag =
            UpsertConflictFragment::new("ethereum_daily_address_transaction_date_uindex");
        conflict_frag
            .add_expression(
                "transaction_count",
                "t.transaction_count + EXCLUDED.transaction_count",
            )
            .add_expression(
                "transaction_volume",
                "t.transaction_volume + EXCLUDED.transaction_volume",
            )
            .add_expression("gas", "t.gas + EXCLUDED.gas");
        self.storage_adapter
            .upsert(&table, &values, &Some(conflict_frag))
    }
}
fn create_columns() -> Vec<Column> {
    create_columns!(
        "address" => ColumnType::String,
        "transaction_date" => ColumnType::Varchar,
        "transaction_count" => ColumnType::BigInt,
        "transaction_volume" => ColumnType::BigDecimal,
        "gas" => ColumnType::BigDecimal
    )
}
fn create_entities(block: &LightEthereumBlock) -> Vec<Entity> {
    let time = UNIX_EPOCH + Duration::from_secs(block.timestamp.as_u64());
    // Create DateTime from SystemTime
    let datetime = chrono::DateTime::<Utc>::from(time)
        .format("%Y-%m-%d")
        .to_string();
    let mut map: BTreeMap<String, (u64, BigInt, BigInt)> = BTreeMap::default();
    block.transactions.iter().for_each(|transaction| {
        let address = format!("{:x}", &transaction.from);
        match map.get_mut(address.as_str()) {
            None => {
                map.insert(
                    address,
                    (
                        1_u64,
                        BigInt::from_unsigned_u256(&transaction.value),
                        BigInt::from_unsigned_u256(&transaction.gas),
                    ),
                );
            }
            Some(tuple) => {
                tuple.0 = tuple.0 + 1;
                tuple.1 = BigInt::from_unsigned_u256(&transaction.value).add(tuple.1.clone());
                tuple.2 = BigInt::from_unsigned_u256(&transaction.gas).add(tuple.2.clone());
            }
        };
    });
    map.iter()
        .map(|(address, tuple)| {
            create_entity!(
                    "address" => address.clone(),
                    "transaction_date" => datetime.clone(),
                    "transaction_count" => tuple.0.clone(),
                    "transaction_volume" => tuple.1.clone(),
                    "gas" => tuple.2.clone()
            )
        })
        .collect::<Vec<Entity>>()
}
