use massbit::prelude::Entity;
use solana_transaction_status::parse_instruction::ParsedInstruction;

//[WIP]
pub fn create_vote_entity(
    _block_slot: u64,
    _tx_hash: String,
    _block_time: u64,
    _inst_order: i32,
    _inst: &ParsedInstruction,
) -> Option<Entity> {
    None
}
