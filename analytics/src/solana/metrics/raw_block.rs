use graph::prelude::{Entity, Attribute, Value};
use massbit_common::NetworkType;
use crate::storage_adapter::StorageAdapter;
use std::sync::Arc;
use crate::relational::{ColumnType, Table, Column};
use std::collections::HashMap;
use graph::data::store::ValueType::BigInt;
use crate::{create_columns,create_entity};
use crate::solana::handler::SolanaHandler;
use massbit_chain_solana::data_type::SolanaBlock;
use solana_transaction_status::ConfirmedBlock;
pub struct SolanaRawBlockHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaRawBlockHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaRawBlockHandler {
            network: network.clone(),
            storage_adapter
        }
    }
}

impl SolanaHandler for SolanaRawBlockHandler {
    fn handle_block(&self, block: &SolanaBlock) -> Result<(), anyhow::Error> {
        let table = Table::new("solana_blocks", Some("t"));
        let columns = create_columns();
        let entity = create_entity(&block.block);
        //println!("Block {:?} has reward {:?}", &block.block.block_height, &block.block.rewards);
        self.storage_adapter.upsert(&table,
                                    &columns,
                                    &vec![entity],
                                    None);
        Ok(())
    }
}
fn create_columns() -> Vec<Column> {
    create_columns!(
        "previous_block_hash" => ColumnType::String,
        "parent_slot" => ColumnType::String,
        "block_hash" => ColumnType::String,
        "block_height" => ColumnType::BigInt,
        "transaction_number" => ColumnType::BigInt,
        "timestamp" => ColumnType::BigInt,
        "leader" => ColumnType::String,
        "reward" => ColumnType::BigInt
    )
}
fn create_entity(block: &ConfirmedBlock) -> Entity {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64
    };
    //block.rewards.iter().reduce(|reward|{ reward.commission});
    //Todo: Calculate leader and reward of the block
    create_entity!(
        "previous_block_hash" => block.previous_blockhash.clone(),
        "parent_slot" => block.parent_slot,
        "block_hash" => block.blockhash.clone(),
        "block_height" => block.block_height,
        "transaction_number" => block.transactions.len() as u64,
        "timestamp" => timestamp,
        "leader" => String::from(""),
        "reward" => 0_u64
    )
}

