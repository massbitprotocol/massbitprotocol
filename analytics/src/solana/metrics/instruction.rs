use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use graph::data::store::scalar::Bytes;
use graph::prelude::{Attribute, Entity, Value};
use massbit_chain_solana::data_type::{Pubkey, SolanaBlock};
use massbit_common::prelude::bs58;
use massbit_common::NetworkType;
use solana_transaction_status::{
    ConfirmedBlock, Reward, RewardType, TransactionWithStatusMeta, UiPartiallyDecodedInstruction,
};
use std::collections::HashMap;
use std::sync::Arc;
pub struct SolanaInstructionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaInstructionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaInstructionHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl SolanaHandler for SolanaInstructionHandler {
    fn handle_block(&self, block: Arc<SolanaBlock>) -> Result<(), anyhow::Error> {
        let table = Table::new("solana_instructions", Some("t"));
        let columns = create_columns();
        let mut entities = Vec::default();
        for tran in &block.block.transactions {
            let mut instruction_entities = create_instructions(&block.block, tran);
            entities.extend(instruction_entities);
            create_inner_instructions(&block.block, tran);
        }
        self.storage_adapter
            .upsert(&table, &columns, &entities, &None);
        Ok(())
    }
}
fn create_columns() -> Vec<Column> {
    create_columns!(
        "block_hash" => ColumnType::String,
        "tx_hash" => ColumnType::String,
        "block_time" => ColumnType::BigInt,
        //Index of instruction in transaction
        "inst_order" => ColumnType::Int,
        "program_name" => ColumnType::String,
        "accounts" => ColumnType::TextArray,
        "data" => ColumnType::Bytes,
        "encoded_data" => ColumnType::String
    )
}
fn create_instructions(block: &ConfirmedBlock, tran: &TransactionWithStatusMeta) -> Vec<Entity> {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64,
    };
    let tx_hash = match tran.transaction.signatures.get(0) {
        Some(sig) => format!("{:?}", sig),
        None => String::from(""),
    };
    let mut entities = Vec::default();
    for (ind, inst) in tran.transaction.message.instructions.iter().enumerate() {
        let program_key = inst.program_id(tran.transaction.message.account_keys.as_slice());
        let accounts = inst
            .accounts
            .iter()
            .filter_map(|&ind| tran.transaction.message.account_keys.get(ind as usize))
            .map(|key| Value::from(key.to_string()))
            .collect::<Vec<Value>>();
        entities.push(create_entity!(
            "block_hash" => block.blockhash.clone(),
            "tx_hash" => tx_hash.clone(),
            "block_time" => timestamp,
            "inst_order" => ind as i32,
            "program_name" => format!("{:?}", program_key),
            "accounts" => accounts,
            "data" => Bytes::from(inst.data.as_slice()),
            "encoded_data" => bs58::encode(&inst.data).into_string()
        ));
    }
    entities
}
fn create_inner_instructions(
    block: &ConfirmedBlock,
    tran: &TransactionWithStatusMeta,
) -> Vec<Entity> {
    tran.meta
        .as_ref()
        .and_then(|meta| meta.inner_instructions.as_ref())
        .and_then(|vec| {
            vec.iter().map(|inner| {
                println!("{:?}", inner);
            });
            Some(0_u64)
        });
    Vec::default()
}
