use crate::relational::Table;
use massbit::prelude::Entity;
use solana_transaction_status::parse_instruction::ParsedInstruction;

pub fn create_spltoken_inst_table(inst_type: &str) -> Option<Table> {
    match inst_type {
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
