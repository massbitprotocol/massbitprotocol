use graph::prelude::Entity;
use solana_transaction_status::parse_instruction::ParsedInstruction;

//[WIP]
pub fn create_vote_entity(
    tx_hash: String,
    block_time: u64,
    inst_order: i32,
    inst: &ParsedInstruction,
) -> Option<Entity> {
    None
}
