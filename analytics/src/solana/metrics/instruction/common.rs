use crate::relational::Table;
use crate::solana::metrics::instruction::system_instruction::create_system_inst_table;
use core::str::FromStr;
use lazy_static::lazy_static;
use massbit_chain_solana::data_type::Pubkey;
use solana_account_decoder::parse_token::spl_token_id_v2_0;
use solana_sdk::{stake, system_program};
use solana_transaction_status::{
    extract_memos::{spl_memo_id_v1, spl_memo_id_v3},
    parse_associated_token::spl_associated_token_id_v1_0,
    parse_instruction::{ParsableProgram, ParsedInstruction},
};
use std::collections::HashMap;

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
    pub static ref PARSABLE_PROGRAM_IDS: HashMap<Pubkey, ParsableProgram> = {
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
    pub fn create_table(&self) -> Option<Table> {
        let program_key = Pubkey::from_str(self.program_id.as_str()).unwrap();
        match PARSABLE_PROGRAM_IDS.get(&program_key) {
            Some(ParsableProgram::System) => create_system_inst_table(self.inst_type.as_str()),
            Some(ParsableProgram::Vote) => None,
            _ => None,
        }
    }
}
