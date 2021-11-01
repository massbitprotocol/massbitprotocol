use crate::relational::{Column, ColumnType, Table};
use crate::{create_columns, create_entity};
use massbit::prelude::{Attribute, Entity, Value};
use solana_transaction_status::parse_instruction::ParsedInstruction;
use std::collections::HashMap;

pub fn create_system_inst_table(inst_type: &str) -> Option<Table> {
    match inst_type {
        "createAccount" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "source" => ColumnType::String,
                "new_account" => ColumnType::String,
                "lamports" => ColumnType::BigInt,
                "space" => ColumnType::BigInt,
                "owner" => ColumnType::String
            );
            Some(Table::new("solana_inst_create_accounts", columns))
        }
        "createAccountWithSeed" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "source" => ColumnType::String,
                "new_account" => ColumnType::String,
                "lamports" => ColumnType::BigInt,
                "space" => ColumnType::BigInt,
                "owner" => ColumnType::String,
                "base" => ColumnType::String,
                "seed" => ColumnType::String
            );
            Some(Table::new("solana_inst_create_accounts", columns))
        }
        "assign" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "account" => ColumnType::String,
                "owner" => ColumnType::String
            );
            Some(Table::new("solana_inst_assigns", columns))
        }
        "assignWithSeed" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "account" => ColumnType::String,
                "owner" => ColumnType::String,
                "base" => ColumnType::String,
                "seed" => ColumnType::String
            );
            Some(Table::new("solana_inst_assigns", columns))
        }
        "transfer" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "source" => ColumnType::String,
                "destination" => ColumnType::String,
                "lamports" => ColumnType::BigInt
            );
            Some(Table::new("solana_inst_transfers", columns))
        }
        "transferWithSeed" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "source" => ColumnType::String,
                "destination" => ColumnType::String,
                "lamports" => ColumnType::BigInt,
                "source_base" => ColumnType::String,
                "source_seed" => ColumnType::String,
                "source_owner" => ColumnType::String
            );
            Some(Table::new("solana_inst_transfers", columns))
        }
        "allocate" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "account" => ColumnType::String,
                "space" => ColumnType::BigInt
            );
            Some(Table::new("solana_inst_allocates", columns))
        }
        "allocateWithSeed" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "account" => ColumnType::String,
                "space" => ColumnType::BigInt,
                "owner" => ColumnType::String,
                "base" => ColumnType::String,
                "seed" => ColumnType::String
            );
            Some(Table::new("solana_inst_allocates", columns))
        }
        "advanceNonce" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "nonce_account" => ColumnType::String,
                "recent_block_hashes_sysvar" => ColumnType::String,
                "nonce_authority" => ColumnType::String
            );
            Some(Table::new("solana_inst_advance_nonces", columns))
        }
        "withdrawFromNonce" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "nonce_account" => ColumnType::String,
                "destination" => ColumnType::String,
                "recent_block_hashes_sysvar" => ColumnType::String,
                "rent_sysvar" => ColumnType::String,
                "nonce_authority" => ColumnType::String,
                "lamports" => ColumnType::BigInt
            );
            Some(Table::new("solana_inst_withdraw_from_nonces", columns))
        }
        "initializeNonce" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "nonce_account" => ColumnType::String,
                "recent_block_hashes_sysvar" => ColumnType::String,
                "rent_sysvar" => ColumnType::String,
                "nonce_authority" => ColumnType::String
            );
            Some(Table::new("solana_inst_initialize_nonces", columns))
        }
        "authorizeNonce" => {
            let columns = create_columns!(
                "tx_hash" => ColumnType::String,
                "block_time" => ColumnType::BigInt,
                "inst_order" => ColumnType::Int,
                "nonce_account" => ColumnType::String,
                "nonce_authority" => ColumnType::String,
                "new_authorized" => ColumnType::String
            );
            Some(Table::new("solana_inst_authorize_nonces", columns))
        }
        _ => None,
    }
}

pub fn create_system_entity(
    _block_slot: u64,
    tx_hash: String,
    block_time: u64,
    inst_order: i32,
    inst: &ParsedInstruction,
) -> Option<Entity> {
    inst.parsed["type"].as_str().and_then(|inst_type| match inst_type {
        a @ "createAccount" | a @ "createAccountWithSeed" => {
            let mut entity = create_entity!(
                "tx_hash" => tx_hash,
                "block_time" => block_time,
                "inst_order" => inst_order,
                "source" => inst.parsed["info"]["source"].as_str().unwrap_or(""),
                "new_account" => inst.parsed["info"]["newAccount"].as_str().unwrap_or(""),
                "lamports" => inst.parsed["info"]["lamports"].as_u64().unwrap_or_default(),
                "space" => inst.parsed["info"]["space"].as_u64().unwrap_or_default(),
                "owner" => inst.parsed["info"]["owner"].as_str().unwrap_or("")
            );
            if "createAccountWithSeed" == a {
                entity.insert(
                    Attribute::from("base"),
                    Value::from(inst.parsed["info"]["base"].as_str().unwrap_or_default()),
                );
                entity.insert(
                    Attribute::from("seed"),
                    Value::from(inst.parsed["info"]["seed"].as_str().unwrap_or_default()),
                );
            }
            Some(entity)
        }
        a @ "assign" | a @ "assignWithSeed" => {
            let mut entity = create_entity!(
                "tx_hash" => tx_hash,
                "block_time" => block_time,
                "inst_order" => inst_order,
                "account" => inst.parsed["info"]["account"].as_str().unwrap_or(""),
                "owner" => inst.parsed["info"]["owner"].as_str().unwrap_or("")
            );
            if "assignWithSeed" == a {
                entity.insert(
                    Attribute::from("base"),
                    Value::from(inst.parsed["info"]["base"].as_str().unwrap_or_default()),
                );
                entity.insert(
                    Attribute::from("seed"),
                    Value::from(inst.parsed["info"]["seed"].as_str().unwrap_or_default()),
                );
            }
            Some(entity)
        }
        a @ "transfer" | a @ "transferWithSeed" => {
            let mut entity = create_entity!(
                "tx_hash" => tx_hash,
                "block_time" => block_time,
                "inst_order" => inst_order,
                "source" => inst.parsed["info"]["source"].as_str().unwrap_or(""),
                "destination" => inst.parsed["info"]["destination"].as_str().unwrap_or(""),
                "lamports" => inst.parsed["info"]["lamports"].as_u64().unwrap_or_default()
            );
            if "transferWithSeed" == a {
                entity.insert(
                    Attribute::from("source_base"),
                    Value::from(
                        inst.parsed["info"]["sourceBase"]
                            .as_str()
                            .unwrap_or_default(),
                    ),
                );
                entity.insert(
                    Attribute::from("source_seed"),
                    Value::from(
                        inst.parsed["info"]["sourceSeed"]
                            .as_str()
                            .unwrap_or_default(),
                    ),
                );
                entity.insert(
                    Attribute::from("source_owner"),
                    Value::from(
                        inst.parsed["info"]["sourceOwner"]
                            .as_str()
                            .unwrap_or_default(),
                    ),
                );
            }
            Some(entity)
        }
        a @ "allocate" | a @ "allocateWithSeed" => {
            let mut entity = create_entity!(
                "tx_hash" => tx_hash,
                "block_time" => block_time,
                "inst_order" => inst_order,
                "account" => inst.parsed["info"]["account"].as_str().unwrap_or(""),
                "space" => inst.parsed["info"]["space"].as_u64().unwrap_or_default()
            );
            if "allocateWithSeed" == a {
                entity.insert(
                    Attribute::from("base"),
                    Value::from(inst.parsed["info"]["base"].as_str().unwrap_or_default()),
                );
                entity.insert(
                    Attribute::from("seed"),
                    Value::from(inst.parsed["info"]["seed"].as_str().unwrap_or_default()),
                );
                entity.insert(
                    Attribute::from("owner"),
                    Value::from(inst.parsed["info"]["owner"].as_str().unwrap_or_default()),
                );
            }
            Some(entity)
        }
        "advanceNonce" => Some(create_entity!(
            "tx_hash" => tx_hash,
            "block_time" => block_time,
            "inst_order" => inst_order,
            "nonce_account" => inst.parsed["info"]["nonceAccount"].as_str().unwrap_or(""),
            "recent_block_hashes_sysvar" => inst.parsed["info"]["recentBlockhashesSysvar"].as_str().unwrap_or(""),
            "nonce_authority" => inst.parsed["info"]["nonceAuthority"].as_str().unwrap_or("")
        )),
        "withdrawFromNonce" => Some(create_entity!(
            "tx_hash" => tx_hash,
            "block_time" => block_time,
            "inst_order" => inst_order,
            "nonce_account" => inst.parsed["info"]["nonceAccount"].as_str().unwrap_or(""),
            "destination" => inst.parsed["info"]["destination"].as_str().unwrap_or(""),
            "recent_block_hashes_sysvar" => inst.parsed["info"]["recentBlockhashesSysvar"].as_str().unwrap_or(""),
            "rent_sysvar" => inst.parsed["info"]["rentSysvar"].as_str().unwrap_or(""),
            "nonce_authority" => inst.parsed["info"]["nonceAuthority"].as_str().unwrap_or(""),
            "lamports" => inst.parsed["info"]["lamports"].as_u64().unwrap_or_default()
        )),
        "initializeNonce" => Some(create_entity!(
            "tx_hash" => tx_hash,
            "block_time" => block_time,
            "inst_order" => inst_order,
            "nonce_account" => inst.parsed["info"]["nonceAccount"].as_str().unwrap_or(""),
            "recent_block_hashes_sysvar" => inst.parsed["info"]["recentBlockhashesSysvar"].as_str().unwrap_or(""),
            "rent_sysvar" => inst.parsed["info"]["rentSysvar"].as_str().unwrap_or(""),
            "nonce_authority" => inst.parsed["info"]["nonceAuthority"].as_str().unwrap_or("")
        )),
        "authorizeNonce" => Some(create_entity!(
            "tx_hash" => tx_hash,
            "block_time" => block_time,
            "inst_order" => inst_order,
            "nonce_account" => inst.parsed["info"]["nonceAccount"].as_str().unwrap_or(""),
            "nonce_authority" => inst.parsed["info"]["nonceAuthority"].as_str().unwrap_or(""),
            "new_authorized" => inst.parsed["info"]["newAuthorized"].as_str().unwrap_or("")
        )),
        _ => None,
    })
}
