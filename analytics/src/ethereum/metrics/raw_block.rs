use crate::schema::*;
use bigdecimal::{BigDecimal, FromPrimitive};
use massbit_chain_ethereum::data_type::{LightEthereumBlock};
use graph::prelude::web3::types::Transaction;
use graph::prelude::{Entity, Attribute, Value};
use crate::ethereum::handler::EthereumHandler;
use massbit_common::NetworkType;
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use crate::relational::{ColumnType, Table, Column};
use std::collections::HashMap;
use graph::data::store::ValueType::BigInt;
use crate::{create_columns,create_entity};
// use index_store::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
// use massbit_drive::{FromEntity, ToMap};

pub struct EthereumRawBlockHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumRawBlockHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumRawBlockHandler {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl EthereumHandler for EthereumRawBlockHandler {
    fn handle_block(&self, block: &LightEthereumBlock) -> Result<(), anyhow::Error> {
        let entity = create_entity(block);
        let table = Table::new("ethereum_block", Some("t"));
        let columns = create_columns();
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &vec![entity],
                                    None);
        Ok(())
    }
}
fn create_columns() -> Vec<Column> {
    create_columns!(
        "block_hash" => ColumnType::String,
        "parent_hash" => ColumnType::String,
        "block_number" => ColumnType::BigInt,
        "transaction_number" => ColumnType::BigInt,
        "timestamp" => ColumnType::BigInt,
        "validated_by" => ColumnType::String,
        "reward" => ColumnType::BigInt,
        "difficulty" => ColumnType::BigInt,
        "total_difficulty" => ColumnType::BigInt,
        "size" => ColumnType::BigInt,
        "gas_used" => ColumnType::BigDecimal,
        "gas_limit" => ColumnType::BigDecimal,
        "extra_data" => ColumnType::Bytes
    )
}
fn create_entity(block: &LightEthereumBlock) -> Entity {
    create_entity!(
        "block_hash" => block.hash,
        "parent_hash" => block.parent_hash,
        "block_number" => block.number,
        "transaction_number" => block.transactions.len() as u64,
        "timestamp" => block.timestamp,
        "validated_by" => block.author,
        "reward" => 0_64,
        "difficulty" => block.difficulty,
        "total_difficulty" => block.total_difficulty,
        "size" => block.size,
        "gas_used" => block.gas_used,
        "gas_limit" => block.gas_limit,
        "extra_data" => block.extra_data.clone()
    )
}

// fn create_entity(block: &LightEthereumBlock) -> Entity {
//     let block_hash = match block.hash {
//         Some(hash) => format!("{:x}",hash),
//         None => String::from("")
//     };
//     let block_number = match block.number {
//         None => 0,
//         Some(val) => val.as_u64() as i64
//     };
//     let timestamp= block.timestamp.as_u64() as i64;
//     let validator = format!("{:x}",block.author);
//     let total_difficulty = match block.total_difficulty {
//         None => BigDecimal::default(),
//         Some(val) => BigDecimal::from_u128(val.as_u128()).unwrap(),
//     };
//     let size = match block.size {
//         None => 0,
//         Some(val) => val.as_u64() as i64
//     };
//     let gas_limit = BigDecimal::from_u128(block.gas_limit.as_u128()).unwrap_or_default();
//     let gas_used = BigDecimal::from_u128(block.gas_used.as_u128()).unwrap_or_default();
//     let mut map : HashMap<Attribute, Value> = HashMap::default();
//     map.insert(Attribute::from("block_hash"), Value::from(block.hash));
//     map.insert(Attribute::from("parent_hash"), Value::from(block.parent_hash));
//     map.insert(Attribute::from("block_number"), Value::from(block.number));
//     map.insert(Attribute::from("transaction_number"), Value::from(block.transactions.len() as u64));
//     map.insert(Attribute::from("timestamp"), Value::from(block.timestamp));
//     map.insert(Attribute::from("validated_by"), Value::from(block.author));
//     map.insert(Attribute::from("reward"), Value::from(0_64));
//     map.insert(Attribute::from("difficulty"), Value::from(block.difficulty));
//     map.insert(Attribute::from("total_difficulty"), Value::from(block.total_difficulty));
//     map.insert(Attribute::from("size"), Value::from(block.size));
//     map.insert(Attribute::from("gas_used"), Value::from(block.gas_used));
//     map.insert(Attribute::from("gas_limit"), Value::from(block.gas_limit));
//     map.insert(Attribute::from("extra_data"), Value::from(block.extra_data.clone()));
//     Entity::from(map)
// }

// #[derive(Default, Debug, Clone, FromEntity, ToMap)]
// pub struct EthereumBlock {
//     pub block_hash: String,
//     pub block_number: i64,
//     pub transaction_number: i64,
//     pub timestamp: i64,
//     pub validated_by: String,
//     pub reward: BigDecimal,
//     pub difficulty: BigDecimal,
//     pub total_difficulty: BigDecimal,
//     pub size: i64,
//     pub gas_used: BigDecimal,
//     pub gas_limit: BigDecimal,
//     pub extra_data: Vec<u8>,
// }