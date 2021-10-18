//[WIP]
use crate::models::CommandData;
use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use massbit::prelude::{Attribute, Entity, Error, Value};
use massbit_chain_solana::data_type::SolanaBlock;
use massbit_common::NetworkType;
use solana_sdk::transaction::Transaction;
use solana_transaction_status::{
    ConfirmedBlock, EncodedConfirmedBlock, TransactionWithStatusMeta, UiTransactionStatusMeta,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub struct SolanaRawTransactionHandler {
    pub network: Option<NetworkType>,
    pub storage_adapter: Arc<dyn StorageAdapter>,
}

impl SolanaRawTransactionHandler {
    pub fn new(network: &Option<NetworkType>, storage_adapter: Arc<dyn StorageAdapter>) -> Self {
        SolanaRawTransactionHandler {
            network: network.clone(),
            storage_adapter,
        }
    }
}

impl SolanaHandler for SolanaRawTransactionHandler {
    fn handle_block(
        &self,
        block_slot: u64,
        block: Arc<EncodedConfirmedBlock>,
    ) -> Result<(), Error> {
        let tran_table = create_trans_table();
        let acc_tran_table = create_acc_trans_table();
        //maximum rows allowed in a query.
        let max_rows = 65535 / acc_tran_table.columns.len();
        let mut tran_entities = Vec::default();
        let mut vec_entities = Vec::default();
        let start = Instant::now();
        for (ind, tran) in block.transactions.iter().enumerate() {
            let decoded_tran = tran.transaction.decode();
            if decoded_tran.is_some() {
                tran_entities.push(create_entity(
                    block_slot,
                    block.clone(),
                    decoded_tran.as_ref().unwrap(),
                    &tran.meta,
                    ind as i32,
                ));
                //Create account trans list
                // let tx_hash = tran
                //     .transaction
                //     .signatures
                //     .get(0)
                //     .and_then(|sig| Some(sig.to_string()));
                let entities = create_transaction_account(
                    block_slot,
                    decoded_tran.as_ref().unwrap(),
                    &tran.meta,
                    ind as i32,
                );
                if entities.len() > 0 {
                    match vec_entities.last_mut() {
                        None => vec_entities.push(entities),
                        Some(last) => {
                            if last.len() + entities.len() <= max_rows {
                                last.extend(entities);
                            } else {
                                vec_entities.push(entities);
                            }
                        }
                    };
                }
            }
        }
        let mut vec_commands = vec_entities
            .iter()
            .map(|entities| CommandData::new(&acc_tran_table, entities, &None))
            .collect::<Vec<CommandData>>();
        if tran_entities.len() > 0 {
            let trans_data = CommandData::new(&tran_table, &tran_entities, &None);
            vec_commands.push(trans_data);
        }
        log::info!(
            "Parsing {} transactions in {:?}",
            tran_entities.len(),
            start.elapsed()
        );
        if vec_commands.len() > 0 {
            self.storage_adapter.transact_upserts(vec_commands)
        } else {
            Ok(())
        }
    }
}
fn create_trans_table<'a>() -> Table<'a> {
    let columns = create_columns!(
        "block_slot" => ColumnType::BigInt,
        "tx_index" => ColumnType::Int,
        "signatures" => ColumnType::String,
        "signers" => ColumnType::String,
        "reward" => ColumnType::BigInt,
        "fee" => ColumnType::BigInt,
        "status" => ColumnType::String
    );
    Table::new("solana_transactions", columns, Some("t"))
}
fn create_acc_trans_table<'a>() -> Table<'a> {
    let columns = create_columns!(
        "block_slot" => ColumnType::BigInt,
        "tx_index" => ColumnType::Int,
        "account" => ColumnType::String,
        "pre_balance" => ColumnType::BigInt,
        "post_balance" => ColumnType::BigInt
    );
    Table::new("solana_account_transactions", columns, Some("t"))
}
fn create_entity(
    block_slot: u64,
    block: Arc<EncodedConfirmedBlock>,
    tran: &Transaction,
    tran_meta: &Option<UiTransactionStatusMeta>,
    ind: i32,
) -> Entity {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64,
    };
    // let signatures = tran.transaction.signatures.iter().map(|sig|{
    //     format!("{:?}", sig)
    // }).collect::<Vec<String>>();
    let tx_hash = tran.signatures.get(0).and_then(|sig| Some(sig.to_string()));

    let mut signers: Vec<String> = Vec::default();
    for i in 0..tran.signatures.len() {
        if let Some(key) = tran.message.account_keys.get(i) {
            signers.push(format!("{:?}", key));
        }
    }

    let mut tran_fee = 0_u64;
    let mut tran_status = String::default();
    if let Some(meta) = tran_meta {
        tran_fee = meta.fee;
        tran_status = match meta.status {
            Ok(_) => "1".to_string(),
            Err(_) => "0".to_string(),
        }
    }
    create_entity!(
        "block_slot" => block_slot,
        "tx_index" => ind,
        "signatures" => tx_hash,
        "signers" => signers.join(","),
        "status" => tran_status,
        "reward" => 0_u64,
        "fee" => tran_fee
    )
}

fn create_transaction_account(
    block_slot: u64,
    tran: &Transaction,
    tran_meta: &Option<UiTransactionStatusMeta>,
    tran_index: i32,
) -> Vec<Entity> {
    //let hash = tx_hash.clone().unwrap_or_default();
    tran.message
        .account_keys
        .iter()
        .enumerate()
        .map(|(ind, key)| {
            let pre_balance = tran_meta
                .as_ref()
                .and_then(|meta| meta.pre_balances.get(ind))
                .unwrap_or(&0_u64);
            let post_balance = tran_meta
                .as_ref()
                .and_then(|meta| meta.post_balances.get(ind))
                .unwrap_or(&0_u64);
            create_entity!(
                "block_slot" => block_slot,
                "tx_index" => tran_index,
                "account" => format!("{:?}", key),
                "pre_balance" => *pre_balance,
                "post_balance" => *post_balance
            )
        })
        .collect::<Vec<Entity>>()
}
