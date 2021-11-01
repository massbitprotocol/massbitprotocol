


use crate::{create_entity};
use massbit::data::store::scalar::Bytes;
use massbit::prelude::{Attribute, Entity, Value};


use solana_sdk::instruction::CompiledInstruction;
use solana_sdk::transaction::Transaction;

use solana_transaction_status::{
    ConfirmedBlock, TransactionWithStatusMeta,
};
use std::collections::HashMap;

pub fn create_unparsed_instruction(
    block_slot: u64,
    tx_index: i32,
    block_time: u64,
    inst_index: i32,
    program_name: String,
    trans: &Transaction,
    inst: &CompiledInstruction,
) -> Entity {
    let mut accounts = Vec::default();
    let mut work = |_unique_ind: usize, acc_ind: usize| {
        match trans
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
            vec.iter().map(|_inner| {
                //println!("{:?}", inner);
            });
            Some(0_u64)
        });
    Vec::default()
}
