use crate::ethereum::handler::EthereumHandler;
use crate::relational::{Column, ColumnType, Table};
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use massbit::components::ethereum::LightEthereumBlock;
use massbit::prelude::{Attribute, Entity, Value};
use massbit_common::NetworkType;
use std::collections::HashMap;
use std::sync::Arc;

pub struct EthereumRawBlockHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl EthereumRawBlockHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        EthereumRawBlockHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl EthereumHandler for EthereumRawBlockHandler {
    fn handle_block(&self, block: Arc<LightEthereumBlock>) -> Result<(), anyhow::Error> {
        let entity = create_entity(block);
        let table = create_table();
        self.storage_adapter.upsert(&table, &vec![entity], &None)
    }
}
fn create_table<'a>() -> Table<'a> {
    let columns = create_columns!(
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
    );
    Table::new("ethereum_blocks", columns, Some("t"))
}
fn create_entity(block: Arc<LightEthereumBlock>) -> Entity {
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
