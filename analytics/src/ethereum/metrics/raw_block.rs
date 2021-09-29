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
