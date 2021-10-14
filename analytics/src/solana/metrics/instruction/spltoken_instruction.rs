use crate::relational::Table;
use massbit::prelude::Entity;
use solana_transaction_status::parse_instruction::ParsedInstruction;

pub fn create_spltoken_inst_table(inst_type: &str) -> Option<Table> {
    match inst_type {
        "initializeMint" => {
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
            Some(Table::new(
                "solana_spl_token_initialize_mint",
                columns,
                Some("t"),
            ))
        }
        _ => None,
    }
}
pub fn create_spltoken_entity(
    block_slot: u64,
    tx_hash: String,
    block_time: u64,
    inst_order: i32,
    inst: &ParsedInstruction,
) -> Option<Entity> {
    None
}
