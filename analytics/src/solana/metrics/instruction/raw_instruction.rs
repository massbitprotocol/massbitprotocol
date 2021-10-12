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
        "block_slot" => ColumnType::BigInt,
        "tx_index" => ColumnType::Int,
        "block_time" => ColumnType::BigInt,
        //Index of instruction in transaction
        "inst_index" => ColumnType::Int,
        "program_name" => ColumnType::String,
        "accounts" => ColumnType::TextArray,
        "data" => ColumnType::Bytes
    );
    Table::new("solana_instructions", columns, Some("t"))
}

pub fn create_unparsed_instruction(
    block_slot: u64,
    tx_index: i32,
    block_time: u64,
    inst_index: i32,
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
        "block_slot" => block_slot,
        "tx_index" => tx_index,
        "block_time" => block_time,
        "inst_index" => inst_index,
        "program_name" => program_name,
        "accounts" => accounts,
        "data" => Bytes::from(inst.data.as_slice())
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
