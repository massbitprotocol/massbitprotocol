use crate::relational::{Column, ColumnType, Table};
use crate::solana::handler::SolanaHandler;
use crate::storage_adapter::StorageAdapter;
use crate::{create_columns, create_entity};
use graph::data::store::scalar::Bytes;
use graph::prelude::{Attribute, Entity, Value};
use inflector::Inflector;
use massbit::prelude::lazy_static;
use massbit_chain_solana::data_type::{Pubkey, SolanaBlock};
use massbit_common::prelude::bs58;
use massbit_common::NetworkType;
use solana_account_decoder::parse_token::spl_token_id_v2_0;
use solana_program::instruction::CompiledInstruction;
use solana_sdk::{stake, system_program};
use solana_transaction_status::{
    extract_memos::{spl_memo_id_v1, spl_memo_id_v3},
    parse_associated_token::{parse_associated_token, spl_associated_token_id_v1_0},
    parse_bpf_loader::{parse_bpf_loader, parse_bpf_upgradeable_loader},
    parse_instruction::{self, ParseInstructionError, ParsedInstruction},
    parse_stake::parse_stake,
    parse_system::parse_system,
    parse_token::parse_token,
    parse_vote::parse_vote,
    ConfirmedBlock, TransactionWithStatusMeta,
};
use std::{
    collections::HashMap,
    str::{from_utf8, Utf8Error},
    sync::Arc,
};

lazy_static! {
    static ref ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = spl_associated_token_id_v1_0();
    static ref BPF_LOADER_PROGRAM_ID: Pubkey = solana_sdk::bpf_loader::id();
    static ref BPF_UPGRADEABLE_LOADER_PROGRAM_ID: Pubkey = solana_sdk::bpf_loader_upgradeable::id();
    static ref MEMO_V1_PROGRAM_ID: Pubkey = spl_memo_id_v1();
    static ref MEMO_V3_PROGRAM_ID: Pubkey = spl_memo_id_v3();
    static ref STAKE_PROGRAM_ID: Pubkey = stake::program::id();
    static ref SYSTEM_PROGRAM_ID: Pubkey = system_program::id();
    static ref TOKEN_PROGRAM_ID: Pubkey = spl_token_id_v2_0();
    static ref VOTE_PROGRAM_ID: Pubkey = solana_vote_program::id();
    static ref PARSABLE_PROGRAM_IDS: HashMap<Pubkey, ParsableProgram> = {
        let mut m = HashMap::new();
        m.insert(
            *ASSOCIATED_TOKEN_PROGRAM_ID,
            ParsableProgram::SplAssociatedTokenAccount,
        );
        m.insert(*MEMO_V1_PROGRAM_ID, ParsableProgram::SplMemo);
        m.insert(*MEMO_V3_PROGRAM_ID, ParsableProgram::SplMemo);
        m.insert(*TOKEN_PROGRAM_ID, ParsableProgram::SplToken);
        m.insert(*BPF_LOADER_PROGRAM_ID, ParsableProgram::BpfLoader);
        m.insert(
            *BPF_UPGRADEABLE_LOADER_PROGRAM_ID,
            ParsableProgram::BpfUpgradeableLoader,
        );
        m.insert(*STAKE_PROGRAM_ID, ParsableProgram::Stake);
        m.insert(*SYSTEM_PROGRAM_ID, ParsableProgram::System);
        m.insert(*VOTE_PROGRAM_ID, ParsableProgram::Vote);
        m
    };
}
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
        let table = Table::new("solana_instructions", Some("t"));
        let columns = create_unparsed_instruction_columns();
        let mut parsed_entities: HashMap<InstructionKey, Vec<Entity>> = HashMap::default();
        let mut unparsed_entities = Vec::default();
        for tran in &block.block.transactions {
            let entities = create_instructions(&block.block, tran);
            parsed_entities.extend(entities.0);
            unparsed_entities.extend(entities.1);
            //create_inner_instructions(&block.block, tran);
        }
        println!("{:?}", &parsed_entities);
        for (key, entities) in parsed_entities.iter() {
            let adapter = self.storage_adapter.clone();
            tokio::spawn(async move {
                let table = key.create_table();
                let columns = key.create_columns();
                adapter.upsert(&table, &columns, entities, &None);
            });
        }
        if unparsed_entities.len() > 0 {
            self.storage_adapter
                .upsert(&table, &columns, &unparsed_entities, &None)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct InstructionKey {
    pub program: String,
    pub program_id: String,
    pub inst_type: String,
}
impl From<&ParsedInstruction> for InstructionKey {
    fn from(inst: &ParsedInstruction) -> Self {
        InstructionKey {
            program: inst.program.clone(),
            program_id: inst.program_id.clone(),
            inst_type: inst.parsed["type"].as_str().unwrap_or_default().to_string(),
        }
    }
}
impl InstructionKey {
    pub fn create_table(&self) -> Table {
        todo!()
    }
    pub fn create_column(&self) -> Vec<Column> {
        todo!()
    }
}
fn create_unparsed_instruction_columns() -> Vec<Column> {
    create_columns!(
        "block_hash" => ColumnType::String,
        "tx_hash" => ColumnType::String,
        "block_time" => ColumnType::BigInt,
        //Index of instruction in transaction
        "inst_order" => ColumnType::Int,
        "program_name" => ColumnType::String,
        "accounts" => ColumnType::TextArray,
        "data" => ColumnType::Bytes,
        "encoded_data" => ColumnType::String
    )
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
) -> Result<Option<Entity>, ParseInstructionError> {
    let program_name = PARSABLE_PROGRAM_IDS
        .get(program_id)
        .ok_or(ParseInstructionError::ProgramNotParsable)?;
    match program_name {
        "system" => Ok(create_system_entity(tx_hash, block_time, inst_order, inst)),
        "vote" => Ok(create_vote_entity(tx_hash, block_time, inst_order, inst)),
        _ => None,
    }
}
fn create_system_entity(
    tx_hash: String,
    block_time: u64,
    inst_order: i32,
    inst: &ParsedInstruction,
) -> Option<Entity> {
    match inst.parsed["type"].as_str() {
        a @ Some("createAccount") | a @ Some("createAccountWithSeed") => {
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
            if Some("createAccountWithSeed") == a {
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
        a @ Some("assign") | a @ Some("assignWithSeed") => {
            let mut entity = create_entity!(
                "tx_hash" => tx_hash,
                "block_time" => block_time,
                "inst_order" => inst_order,
                "account" => inst.parsed["info"]["account"].as_str().unwrap_or(""),
                "owner" => inst.parsed["info"]["owner"].as_str().unwrap_or("")
            );
            if Some("assignWithSeed") == a {
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
        a @ Some("transfer") | a @ Some("transferWithSeed") => {
            let mut entity = create_entity!(
                "tx_hash" => tx_hash,
                "block_time" => block_time,
                "inst_order" => inst_order,
                "source" => inst.parsed["info"]["source"].as_str().unwrap_or(""),
                "destination" => inst.parsed["info"]["destination"].as_str().unwrap_or(""),
                "lamports" => inst.parsed["info"]["lamports"].as_u64().unwrap_or_default()
            );
            if Some("transferWithSeed") == a {
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
        a @ Some("allocate") | a @ Some("allocateWithSeed") => {
            let mut entity = create_entity!(
                "tx_hash" => tx_hash,
                "block_time" => block_time,
                "inst_order" => inst_order,
                "account" => inst.parsed["info"]["account"].as_str().unwrap_or(""),
                "space" => inst.parsed["info"]["space"].as_u64().unwrap_or_default()
            );
            if Some("allocateWithSeed") == a {
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
        Some("advanceNonce") => Some(create_entity!(
            "tx_hash" => tx_hash,
            "block_time" => block_time,
            "inst_order" => inst_order,
            "nonce_account" => inst.parsed["info"]["nonceAccount"].as_str().unwrap_or(""),
            "recent_block_hashes_sysvar" => inst.parsed["info"]["recentBlockhashesSysvar"].as_str().unwrap_or(""),
            "nonce_authority" => inst.parsed["info"]["nonceAuthority"].as_str().unwrap_or("")
        )),
        Some("withdrawFromNonce") => Some(create_entity!(
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
        Some("initializeNonce") => Some(create_entity!(
            "tx_hash" => tx_hash,
            "block_time" => block_time,
            "inst_order" => inst_order,
            "nonce_account" => inst.parsed["info"]["nonceAccount"].as_str().unwrap_or(""),
            "recent_block_hashes_sysvar" => inst.parsed["info"]["recentBlockhashesSysvar"].as_str().unwrap_or(""),
            "rent_sysvar" => inst.parsed["info"]["rentSysvar"].as_str().unwrap_or(""),
            "nonce_authority" => inst.parsed["info"]["nonceAuthority"].as_str().unwrap_or("")
        )),
        Some("authorizeNonce") => Some(create_entity!(
            "tx_hash" => tx_hash,
            "block_time" => block_time,
            "inst_order" => inst_order,
            "nonce_account" => inst.parsed["info"]["nonceAccount"].as_str().unwrap_or(""),
            "nonce_authority" => inst.parsed["info"]["nonceAuthority"].as_str().unwrap_or(""),
            "new_authorized" => inst.parsed["info"]["newAuthorized"].as_str().unwrap_or("")
        )),
        _ => None,
    }
}
fn create_vote_entity(
    tx_hash: String,
    block_time: u64,
    inst_order: i32,
    inst: &ParsedInstruction,
) -> Option<Entity> {
    None
}

fn create_unparsed_instruction(
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
