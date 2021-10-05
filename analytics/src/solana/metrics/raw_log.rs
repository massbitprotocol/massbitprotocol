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

pub struct SolanaRawLogHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaRawLogHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaRawLogHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl SolanaHandler for SolanaRawLogHandler {
    fn handle_block(&self, block: Arc<SolanaBlock>) -> Result<(), anyhow::Error> {
        let table = Table::new("solana_logs", Some("t"));
        let columns = create_columns();
        let entities = block
            .block
            .transactions
            .iter()
            .filter_map(|tran| {
                tran.meta
                    .as_ref()
                    .and_then(|meta| meta.log_messages.as_ref())
                    .and_then(|logs| Some(create_entity(&block.block, tran, logs)))
            })
            .collect::<Vec<Entity>>();
        self.storage_adapter
            .upsert(&table, &columns, &entities, &None);
        Ok(())
    }
}

fn create_columns() -> Vec<Column> {
    create_columns!(
        "tx_hash" => ColumnType::String,
        "log_messages" => ColumnType::TextArray,
        "block_time" => ColumnType::BigInt
    )
}
fn create_entity(
    block: &ConfirmedBlock,
    tran: &TransactionWithStatusMeta,
    logs: &Vec<String>,
) -> Entity {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64,
    };
    let tx_hash = match tran.transaction.signatures.get(0) {
        Some(sig) => format!("{:?}", sig),
        None => String::from(""),
    };
    let messages = logs
        .iter()
        .map(|msg| Value::from(msg.clone()))
        .collect::<Vec<Value>>();
    create_entity!(
        "tx_hash" => tx_hash,
        "log_messages" => messages,
        "block_time" => timestamp
    )
}
