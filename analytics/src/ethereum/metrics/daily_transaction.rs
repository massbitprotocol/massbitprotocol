use crate::ethereum::handler::EthereumHandler;
use massbit_chain_ethereum::data_type::{ExtBlock, LightEthereumBlock};
use graph::prelude::web3::types::{Transaction, TransactionReceipt, U256};
use graph::prelude::{Value, BigInt, BigDecimal as BigDecimalValue, Attribute};
use graph::prelude::bigdecimal::{BigDecimal, FromPrimitive};
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use massbit_common::NetworkType;
use graph::prelude::chrono;
use std::time::UNIX_EPOCH;
use std::time::Duration;
use graph::prelude::chrono::Utc;
//use schema::ethereum_daily_transaction;
use massbit_common::prelude::diesel::result::Error;
use massbit_common::prelude::diesel::RunQueryDsl;
use std::collections::HashMap;
use graph::data::store::Entity;
use crate::relational::{Column, ColumnType, Table};
use crate::util::timestamp_round_to_date;
use crate::postgres_queries::UpsertConflictFragment;
use crate::{create_columns, create_entity};
pub struct EthereumDailyTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,

}
impl EthereumDailyTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumDailyTransactionHandler {
            network: network.clone(),
            storage_adapter
        }
    }
}
impl EthereumHandler for EthereumDailyTransactionHandler {
    fn handle_block(&self, block: &LightEthereumBlock) -> Result<(), anyhow::Error> {

        let table = Table::new("ethereum_daily_transaction", Some("t"));
        let columns = create_columns();
        let entity = create_entity(self.network.clone(), block);
        let mut conflict_frag = UpsertConflictFragment::new("ethereum_daily_transaction_transaction_date_network_uindex");
        conflict_frag.add_expression("transaction_count", "t.transaction_count + EXCLUDED.transaction_count")
            .add_expression("transaction_volume","t.transaction_volume + EXCLUDED.transaction_volume")
            .add_expression("gas","t.gas + EXCLUDED.gas")
            .add_expression("average_gas_price","(t.average_gas_price * t.transaction_count + EXCLUDED.average_gas_price * EXCLUDED.transaction_count)\
                    /(t.transaction_count + EXCLUDED.transaction_count)");
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &vec![entity],
                                    Some(conflict_frag));
        Ok(())
    }
}

fn create_columns() -> Vec<Column> {
    create_columns!(
        "network" => ColumnType::String,
        "transaction_date" => ColumnType::Varchar,
        "transaction_count" => ColumnType::BigInt,
        "transaction_volume" => ColumnType::BigDecimal,
        "gas" => ColumnType::BigInt,
        "average_gas_price" => ColumnType::BigDecimal
    )
}
fn create_entity(network_name: Option<NetworkType>, block: &LightEthereumBlock) -> Entity {
    //let timestamp: u64 = block.timestamp.as_u64();
    let timestamp = timestamp_round_to_date(block.timestamp.as_u64());
    let time = UNIX_EPOCH + Duration::from_secs(block.timestamp.as_u64());
    // Create DateTime from SystemTime
    let datetime = chrono::DateTime::<Utc>::from(time);
    let date = datetime.format("%Y-%m-%d").to_string();

    let gas_used = BigDecimal::from_u128(block.gas_used.as_u128());
    let gas_limit = BigDecimal::from_u128(block.gas_limit.as_u128());
    let size = match block.size {
        None => None,
        Some(val) => BigDecimal::from_u128(val.as_u128())
    };
    let transaction_count = block.transactions.len() as u64;
    let (transaction_volume, gas_price, gas) = block.transactions.iter().fold((BigDecimal::default(), BigDecimal::default(), BigDecimal::default()), |acc, tran| {
        let value = match BigDecimal::from_u128(tran.value.as_u128()) {
            None => acc.0,
            Some(val) => acc.0 + val
        };
        let gas = match BigDecimal::from_u128(tran.gas.as_u128()) {
            None => acc.1,
            Some(val) => acc.1 + val
        };
        let gas_price = match BigDecimal::from_u128(tran.gas_price.as_u128()) {
            None => acc.2,
            Some(val) => acc.2 + val
        };
        (value, gas, gas_price)
    });

    let mut row_value : HashMap<Attribute, Value> = HashMap::default();
    if network_name.is_none() {
         row_value.insert(Attribute::from("network"), Value::Null);
    } else {
         row_value.insert(Attribute::from("network"), Value::String(network_name.as_ref().unwrap().clone()));
    }
    row_value.insert(Attribute::from("transaction_date"), Value::String(date));
    row_value.insert(Attribute::from("transaction_count"), Value::BigInt(BigInt::from(transaction_count as u64)));
    row_value.insert(Attribute::from("transaction_volume"), Value::BigDecimal(BigDecimalValue::from(transaction_volume)));
    row_value.insert(Attribute::from("gas"), Value::BigDecimal(BigDecimalValue::from(gas)));
    let average_gas_price : BigDecimal = if transaction_count > 0 {gas_price/ BigDecimal::from_u64(transaction_count).unwrap()} else { BigDecimal::default() };
    row_value.insert(Attribute::from("average_gas_price"), Value::BigDecimal(BigDecimalValue::from(average_gas_price)));
    Entity::from(row_value)
    // create_entity!(
    //     "network" => network_name,
    //     "transaction_date" => date,
    //     "transaction_count" => transaction_count,
    //     "transaction_volume" => transaction_volume,
    //     "gas" => gas,
    //     "average_gas_price" => average_gas_price
    // )
}
