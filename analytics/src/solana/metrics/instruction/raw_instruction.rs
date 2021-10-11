use crate::relational::{Column, ColumnType, Table};
use crate::solana::metrics::instruction::common::InstructionKey;
use crate::solana::metrics::instruction::system_instruction::create_system_entity;
use crate::{create_columns, create_entity};
use massbit::data::store::scalar::Bytes;
use massbit::prelude::{Attribute, Entity, Value};
use massbit_chain_solana::data_type::Pubkey;
use massbit_common::prelude::bs58;
use solana_sdk::instruction::CompiledInstruction;
use solana_transaction_status::parse_instruction::{ParsableProgram, ParsedInstruction};
use solana_transaction_status::{ConfirmedBlock, TransactionWithStatusMeta};
use std::collections::HashMap;

pub fn create_unparsed_instruction_table<'a>() -> Table<'a> {
    let columns = create_columns!(
        "block_hash" => ColumnType::String,
        "tx_hash" => ColumnType::String,
        "block_time" => ColumnType::BigInt,
        //Index of instruction in transaction
        "inst_order" => ColumnType::Int,
        "program_name" => ColumnType::String,
        "accounts" => ColumnType::TextArray,
        "data" => ColumnType::Bytes,
        "encoded_data" => ColumnType::String
    );
    Table::new("solana_instructions", columns, Some("t"))
}

pub fn create_unparsed_instruction(
    block_hash: String,
    tx_hash: String,
    block_time: u64,
    inst_order: i32,
    program_name: String,
    trans: &TransactionWithStatusMeta,
    inst: &CompiledInstruction,
) -> Entity {
    let mut accounts = Vec::default();
    let mut work = |unique_ind: usize, acc_ind: usize| {
        match trans
            .transaction
            .message
            .account_keys
            .get(acc_ind)
            .map(|key| Value::from(key.to_string()))
        {
            None => {}
            Some(val) => accounts.push(val),
        };
        Ok(())
    };

    inst.visit_each_account(&mut work);
    create_entity!(
        "block_hash" => block_hash,
        "tx_hash" => tx_hash,
        "block_time" => block_time,
        "inst_order" => inst_order,
        "program_name" => program_name,
        "accounts" => accounts,
        "data" => Bytes::from(inst.data.as_slice()),
        "encoded_data" => bs58::encode(&inst.data).into_string()
    )
}
fn create_inner_instructions(
    _block: &ConfirmedBlock,
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
