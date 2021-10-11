use crate::postgres_queries::UpsertConflictFragment;
use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use massbit::prelude::{Attribute, Entity, Value};
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::NetworkType;
use solana_transaction_status::RewardType;
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
        let table = create_table();
        let network = match &self.network {
            None => "",
            Some(val) => val.as_str(),
        };
        let entity = create_stat_block_entity(network, block);
        let mut conflict_frag = UpsertConflictFragment::new("solana_daily_stat_block_date_uindex");
        conflict_frag.add_expression("min_block_height", "LEAST(t.min_block_height, EXCLUDED.min_block_height)")
            .add_expression("max_block_height", "GREATEST(t.max_block_height, EXCLUDED.max_block_height)")
            .add_expression("block_counter","t.block_counter + EXCLUDED.block_counter")
            .add_expression("total_tx","t.total_tx + EXCLUDED.total_tx")
            .add_expression("success_tx","t.success_tx + EXCLUDED.success_tx")
            .add_expression("total_fee","t.total_fee + EXCLUDED.total_fee")
            .add_expression("total_reward","t.total_reward + EXCLUDED.total_reward")
            //First block in current day
            .add_expression("fist_block_time","LEAST(t.fist_block_time, EXCLUDED.fist_block_time)")
            //latest incoming block
            .add_expression("last_block_time","GREATEST(t.last_block_time, EXCLUDED.last_block_time)")
            //Average block time in ms
            .add_expression("average_block_time","(GREATEST(t.last_block_time, EXCLUDED.last_block_time) - LEAST(t.fist_block_time, EXCLUDED.fist_block_time))\
                    * 1000 /(GREATEST(t.max_block_height, EXCLUDED.max_block_height) - LEAST(t.min_block_height, EXCLUDED.min_block_height) + 1)");
        self.storage_adapter
            .upsert(&table, &vec![entity], &Some(conflict_frag))
    }
}

fn create_table<'a>() -> Table<'a> {
    let columns = create_columns!(
        "network" => ColumnType::String,
        "date" => ColumnType::BigInt,
        "min_block_height" => ColumnType::BigInt,
        "max_block_height" => ColumnType::BigInt,
        "block_counter" => ColumnType::BigInt,
        "total_tx" => ColumnType::BigInt,
        "success_tx" => ColumnType::BigInt,
        "total_reward" => ColumnType::BigInt,
        "total_fee" => ColumnType::BigInt,
        "fist_block_time" => ColumnType::BigInt,
        "last_block_time" => ColumnType::BigInt
    );
    Table::new("solana_daily_stat_block", columns, Some("t"))
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
    //Sum success transactions' fee and count success transaction

    let success_trans = block
        .block
        .transactions
        .iter()
        .filter_map(|tran| {
            tran.meta.as_ref().and_then(|meta| match meta.status {
                Ok(_) => Some((meta.fee, 1_u64)),
                Err(_) => None,
            })
        })
        .reduce(|a, b| (a.0 + b.0, a.1 + b.1));
    let mut total_fee = 0_u64;
    let mut counter = 0_u64;
    if let Some(val) = success_trans {
        total_fee = val.0;
        counter = val.1;
    }

    create_entity!(
        "network" => network.to_string(),
        "date" => date,
        "min_block_height" => block_height,
        "max_block_height" => block_height,
        "block_counter" => 1_u64,
        "total_tx" => block.block.transactions.len() as u64,
        "success_tx" => counter,
        "total_reward" => reward_val,
        "total_fee" => total_fee,
        "fist_block_time" => block_time,
        "last_block_time" => block_time
    )
}
