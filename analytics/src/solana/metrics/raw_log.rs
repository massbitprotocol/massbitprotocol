use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use massbit::prelude::{Attribute, Entity, Error, Value};
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::NetworkType;
use solana_transaction_status::{
    ConfirmedBlock, EncodedConfirmedBlock, EncodedTransactionWithStatusMeta,
    TransactionWithStatusMeta, UiTransactionStatusMeta,
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
    fn handle_block(
        &self,
        block_slot: u64,
        block: Arc<EncodedConfirmedBlock>,
    ) -> Result<(), Error> {
        let table = create_table();
        let entities = block
            .transactions
            .iter()
            .filter_map(|tran| {
                tran.meta
                    .as_ref()
                    .and_then(|meta| meta.log_messages.as_ref())
                    .and_then(|logs| Some(create_entity(block_slot, &block, tran, logs)))
            })
            .collect::<Vec<Entity>>();
        if entities.len() > 0 {
            self.storage_adapter.upsert(&table, &entities, &None)
        } else {
            Ok(())
        }
    }
}

fn create_table<'a>() -> Table<'a> {
    let columns = create_columns!(
        "tx_hash" => ColumnType::String,
        "log_messages" => ColumnType::TextArray,
        "block_time" => ColumnType::BigInt
    );
    Table::new("solana_logs", columns, Some("t"))
}
fn create_entity(
    block_slot: u64,
    block: &EncodedConfirmedBlock,
    tran: &EncodedTransactionWithStatusMeta,
    logs: &Vec<String>,
) -> Entity {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64,
    };
    let tx_hash = tran
        .transaction
        .decode()
        .as_ref()
        .and_then(|decoded_tran| decoded_tran.signatures.get(0))
        .and_then(|sig| Some(format!("{:?}", sig)))
        .unwrap_or_default();
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
