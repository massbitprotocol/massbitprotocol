use super::common::PARSABLE_PROGRAM_IDS;
use crate::solana::handler::SolanaHandler;
use crate::solana::metrics::instruction::common::InstructionKey;
use crate::solana::metrics::instruction::raw_instruction::{
    create_unparsed_instruction, create_unparsed_instruction_table,
};
use crate::solana::metrics::instruction::spltoken_instruction::create_spltoken_entity;
use crate::solana::metrics::instruction::system_instruction::create_system_entity;
use crate::solana::metrics::instruction::vote_instruction::create_vote_entity;
use crate::storage_adapter::StorageAdapter;
use massbit::prelude::Entity;
use massbit_chain_solana::data_type::{Pubkey, SolanaBlock};
use massbit_common::NetworkType;
use solana_transaction_status::parse_instruction::{ParsableProgram, ParsedInstruction};
use solana_transaction_status::{parse_instruction, ConfirmedBlock, TransactionWithStatusMeta};
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
        let table = create_unparsed_instruction_table();
        let mut parsed_entities: HashMap<InstructionKey, Vec<Entity>> = HashMap::default();
        //let mut unparsed_entities = Vec::default();
        for tran in &block.block.transactions {
            let entities = create_instructions(&block.block, tran);
            parsed_entities.extend(entities.0);
            //unparsed_entities.extend(entities.1);
            //create_inner_instructions(&block.block, tran);
        }
        let arc_map_entities = Arc::new(parsed_entities);
        arc_map_entities.iter().for_each(|(key, _)| {
            let adapter = self.storage_adapter.clone();
            let cloned_map = arc_map_entities.clone();
            let cloned_key = key.clone();
            tokio::spawn(async move {
                match cloned_key.create_table() {
                    Some(table) => {
                        adapter.upsert(&table, cloned_map.get(&cloned_key).unwrap(), &None);
                    }
                    None => {}
                }
            });
        });
        Ok(())
        //Don't store unpased instruction due to huge amount of data
        // if unparsed_entities.len() > 0 {
        //     self.storage_adapter
        //         .upsert(&table, &unparsed_entities, &None)
        // } else {
        //     Ok(())
        // }
    }
}

///
/// For each transaction try to parse instructions and create correspond entities,
/// Unparsed instructions are converted to common entities
///
fn create_instructions(
    block: &ConfirmedBlock,
    tran: &TransactionWithStatusMeta,
) -> (HashMap<InstructionKey, Vec<Entity>>, Vec<Entity>) {
    let timestamp = match block.block_time {
        None => 0_u64,
        Some(val) => val as u64,
    };
    let tx_hash = match tran.transaction.signatures.get(0) {
        Some(sig) => format!("{:?}", sig),
        None => String::from(""),
    };
    let mut unparsed_instruactions = Vec::default();
    let mut parsed_instrucions: HashMap<InstructionKey, Vec<Entity>> = HashMap::default();
    for (ind, inst) in tran.transaction.message.instructions.iter().enumerate() {
        let program_key = inst.program_id(tran.transaction.message.account_keys.as_slice());
        match parse_instruction::parse(
            program_key,
            inst,
            tran.transaction.message.account_keys.as_slice(),
        ) {
            Ok(parsed_inst) => {
                let key = InstructionKey::from(&parsed_inst);
                if let Some(entity) = create_parsed_entity(
                    program_key,
                    tx_hash.clone(),
                    timestamp,
                    ind as i32,
                    &parsed_inst,
                ) {
                    match parsed_instrucions.get_mut(&key) {
                        None => {
                            parsed_instrucions.insert(key, vec![entity]);
                        }
                        Some(vec) => {
                            vec.push(entity);
                        }
                    };
                };
            }
            Err(_) => {
                unparsed_instruactions.push(create_unparsed_instruction(
                    block.blockhash.clone(),
                    tx_hash.clone(),
                    timestamp,
                    ind as i32,
                    program_key.to_string(),
                    tran,
                    inst,
                ));
            }
        }
    }
    (parsed_instrucions, unparsed_instruactions)
}

fn create_parsed_entity(
    program_id: &Pubkey,
    tx_hash: String,
    block_time: u64,
    inst_order: i32,
    inst: &ParsedInstruction,
) -> Option<Entity> {
    match PARSABLE_PROGRAM_IDS.get(program_id) {
        Some(ParsableProgram::System) => {
            create_system_entity(tx_hash, block_time, inst_order, inst)
        }
        Some(ParsableProgram::SplToken) => {
            create_spltoken_entity(tx_hash, block_time, inst_order, inst)
        }
        Some(ParsableProgram::Vote) => create_vote_entity(tx_hash, block_time, inst_order, inst),
        _ => None,
    }
}
