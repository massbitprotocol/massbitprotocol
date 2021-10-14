use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use massbit::prelude::{Attribute, Entity, Value};
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::NetworkType;
use solana_transaction_status::{ConfirmedBlock, RewardType};
use std::collections::HashMap;
use std::sync::Arc;
pub struct SolanaRawBlockHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaRawBlockHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaRawBlockHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl SolanaHandler for SolanaRawBlockHandler {
    fn handle_block(&self, block_slot: u64, block: Arc<SolanaBlock>) -> Result<(), anyhow::Error> {
        let table = create_table();
        let entity = create_entity(block_slot, &block.block);
        //println!("Block {:?} has reward {:?}", &block.block.block_height, &block.block.rewards);
        self.storage_adapter.upsert(&table, &vec![entity], &None)
    }
}
fn create_table<'a>() -> Table<'a> {
    let columns = create_columns!(
        "block_slot" => ColumnType::BigInt,
        "previous_block_hash" => ColumnType::String,
        "parent_slot" => ColumnType::BigInt,
        "block_hash" => ColumnType::String,
        "transaction_number" => ColumnType::Int,
        "timestamp" => ColumnType::BigInt,
        "leader" => ColumnType::String,
        "reward" => ColumnType::BigInt
    );
    Table::new("solana_blocks", columns, Some("t"))
}
fn create_entity(block_slot: u64, block: &ConfirmedBlock) -> Entity {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64,
    };
    //Calculate leader and reward of the block ad reward with tye Fee
    let mut validator = String::from("");
    let mut reward_val = 0_u64;
    for reward in &block.rewards {
        if Some(RewardType::Fee) == reward.reward_type {
            validator = reward.pubkey.clone();
            reward_val = reward.lamports as u64;
            break;
        }
    }
    create_entity!(
        "block_slot" => block_slot,
        "previous_block_hash" => block.previous_blockhash.clone(),
        "parent_slot" => block.parent_slot,
        "block_hash" => block.blockhash.clone(),
        "transaction_number" => block.transactions.len() as i32,
        "timestamp" => timestamp,
        "leader" => validator,
        "reward" => reward_val
    )
}
