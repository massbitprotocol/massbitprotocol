use crate::postgres_queries::UpsertConflictFragment;
use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use graph::data::schema::{FulltextAlgorithm, FulltextConfig, FulltextLanguage};
use graph::data::store::ValueType::BigInt;
use graph::prelude::{Attribute, Entity, Value};
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::NetworkType;
use solana_transaction_status::{
    ConfirmedBlock, Reward, RewardType, TransactionStatusMeta, TransactionWithStatusMeta,
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SolanaStatBlockHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaStatBlockHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaStatBlockHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl SolanaHandler for SolanaStatBlockHandler {
    fn handle_block(&self, block: Arc<SolanaBlock>) -> Result<(), anyhow::Error> {
        let table = Table::new("solana_daily_stat_block", Some("t"));
        let columns = create_columns();
        let network = match &self.network {
            None => "",
            Some(val) => val.as_str(),
        };
        let entity = create_stat_block_entity(network, block);
        let mut conflict_frag = UpsertConflictFragment::new("solana_daily_stat_block_date_uindex");
        conflict_frag.add_expression("min_block_height", "LEAST(t.min_block_height, EXCLUDED.min_block_height)")
            .add_expression("max_block_height", "GREATEST(t.max_block_height, EXCLUDED.max_block_height)")
            .add_expression("transaction_counter","t.transaction_counter + EXCLUDED.transaction_counter")
            .add_expression("average_reward","(t.average_reward * t.transaction_counter + EXCLUDED.average_reward * EXCLUDED.transaction_counter)\
                    /(t.transaction_counter + EXCLUDED.transaction_counter)")
            //First block in current day
            .add_expression("fist_block_time","LEAST(t.fist_block_time, EXCLUDED.fist_block_time)")
            //latest incoming block
            .add_expression("last_block_time","GREATEST(t.last_block_time, EXCLUDED.last_block_time)")
            //Average block time in ms
            .add_expression("average_block_time","(GREATEST(t.last_block_time, EXCLUDED.last_block_time) - LEAST(t.fist_block_time, EXCLUDED.fist_block_time))\
                    * 1000 /(GREATEST(t.max_block_height, EXCLUDED.max_block_height) - LEAST(t.min_block_height, EXCLUDED.min_block_height))");
        self.storage_adapter
            .upsert(&table, &columns, &vec![entity], &Some(conflict_frag));
        Ok(())
    }
}

fn create_columns() -> Vec<Column> {
    create_columns!(
        "network" => ColumnType::String,
        "date" => ColumnType::BigInt,
        "min_block_height" => ColumnType::BigInt,
        "max_block_height" => ColumnType::BigInt,
        "transaction_counter" => ColumnType::BigInt,
        "average_reward" => ColumnType::BigInt,
        "fist_block_time" => ColumnType::BigInt,
        "last_block_time" => ColumnType::BigInt
    )
}
fn create_stat_block_entity(network: &str, block: Arc<SolanaBlock>) -> Entity {
    //Make timestamp as multiple of a day's seconds
    let block_time = match block.block.block_time {
        None => 0_u64,
        Some(val) => val as u64,
    };
    let block_height = block.block.block_height.unwrap_or_default();
    let date = block_time / 86400 * 86400;
    let mut reward_val = 0_u64;
    for reward in &block.block.rewards {
        if Some(RewardType::Fee) == reward.reward_type {
            reward_val = reward.lamports as u64;
            break;
        }
    }
    create_entity!(
        "network" => network.to_string(),
        "date" => date,
        "min_block_height" => block_height,
        "max_block_height" => block_height,
        "transaction_counter" => block.block.transactions.len() as u64,
        "average_reward" => reward_val,
        "fist_block_time" => block_time,
        "last_block_time" => block_time
    )
}
