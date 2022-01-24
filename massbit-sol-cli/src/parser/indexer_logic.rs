use crate::config::IndexerConfig;
use crate::parser::{Definitions, InstructionHandler, Visitor};
use crate::schema::AccountInfo;
use std::collections::HashMap;
use syn::File;

pub struct IndexerLogicGenerator<'a> {
    pub config: IndexerConfig,
    definitions: &'a Definitions,
    variant_accounts: &'a HashMap<String, Vec<AccountInfo>>,
}
impl<'a> IndexerLogicGenerator<'a> {
    pub fn new(
        config: IndexerConfig,
        definitions: &'a Definitions,
        variant_accounts: &'a HashMap<String, Vec<AccountInfo>>,
    ) -> Self {
        Self {
            config,
            definitions,
            variant_accounts,
        }
    }
    pub fn generate(&self, ast: &File) {
        //Cargo toml
        self.gen_cargo_toml();
        //mapping.rs
        self.gen_mapping();
        //lib.rs
        self.gen_lib();
        //Handler
        let output_path = format!("{}/src/generated/mod.rs", &self.config.output_logic);
        match std::fs::write(&output_path, "pub mod handler;") {
            Ok(_) => {
                log::info!("Write file mod.rs success full");
            }
            Err(err) => {
                log::error!("{:?}", &err);
            }
        }
        let mut handler =
            InstructionHandler::new(self.config.clone(), self.definitions, self.variant_accounts);
        handler.visit_file(ast);
        handler.write_output("handler.rs");
    }
    pub fn gen_mapping(&self) {
        let output_path = format!("{}/src/mapping.rs", &self.config.output_logic);
        let content = format!(
            r#"use std::sync::Arc;
use massbit_solana_sdk::smart_contract::{{InstructionParser, SmartContractProxy}};
use massbit_solana_sdk::types::SolanaBlock;
use crate::generated::handler::Handler;
use crate::ADDRESS;
use crate::SOLANA_CLIENT;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_client::rpc_response::RpcResult;
use solana_client::{{client_error::Result as ClientResult, rpc_request::RpcRequest}};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::CompiledInstruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use solana_transaction_status::{{parse_instruction, ConfirmedBlock, TransactionWithStatusMeta}};
use massbit_solana_sdk::transport::interface::InterfaceRegistrar;
use uuid::Uuid;


pub fn handle_block(interface: &mut dyn InstructionParser, block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {{
    println!("Start handle_block, block.block_number: {{}}", block.block_number);
    for (tx_ind, tran) in block.block.transactions.iter().enumerate() {{
        if tran
            .transaction
            .message
            .account_keys
            .iter()
            .any(|key| key.to_string().as_str() == ADDRESS)
        {{
            let entities = parse_instructions(interface, block, tran, tx_ind);
        }}
    }}
    Ok(())
}}
fn parse_instructions(interface: &mut dyn InstructionParser, block: &SolanaBlock, tran: &TransactionWithStatusMeta, tx_ind: usize) {{
    for (ind, inst) in tran.transaction.message.instructions.iter().enumerate() {{
        let program_key = inst.program_id(tran.transaction.message.account_keys.as_slice());
        if program_key.to_string().as_str() == ADDRESS {{
            let mut accounts = Vec::default();
            let mut work = |unique_ind: usize, acc_ind: usize| {{
                if let Some(key) = tran.transaction.message.account_keys.get(acc_ind) {{
                    accounts.push(key.clone());
                }};
                Ok(())
            }};
            inst.visit_each_account(&mut work);

            let handler = Handler {{}};
            // Fixme: Get account_infos from chain take a lot of time. For now, use empty vector.
            println!("Start unpack_instruction, inst {{:?}}", &inst);
            match interface.unpack_instruction(inst.data.as_slice()) {{
                Ok(trans_value) => {{
                    println!("unpack_instruction Ok, trans_value: {{:?}}", &trans_value);
                    handler.process(block, tran, program_key, &accounts, trans_value);
                }},
                Err(e) => {{
                    println!("Error unpack_instruction: {{:?}}",e);
                }}
            }}
        }}
    }}
}}
            "#
        );
        match std::fs::write(&output_path, &content) {
            Ok(res) => {
                log::info!("Write file cargo.toml success full");
            }
            Err(err) => {
                log::error!("{:?}", &err);
            }
        }
    }
    fn gen_cargo_toml(&self) {
        if std::fs::create_dir_all(&self.config.output_logic).is_ok() {
            let output_path = format!("{}/Cargo.toml", &self.config.output_logic);
            let content = format!(
                r#"[package]
name = "indexer-logic"
version = "0.0.1"
description = "Indexer logic"
authors = ["Maintainers <contact@massbit.io>"]
repository = "https://github.com/massbitprotocol/solana-indexer-examples.git"
license = "Apache-2.0"
edition = "2018"

[dependencies]
diesel = {{ version = "1.4.0", features = ["postgres"] }}
chrono = "0.4.19"
hex = "0.4.3"
anyhow = "1.0.44"
uuid = {{ version = "0.8", features = ["serde", "v4"] }}
arbitrary = {{ version = "0.4.6", features = ["derive"], optional = true }}
log = "0.4.14"
num_enum = "0.5.0"
thiserror = "1.0.20"
safe-transmute = "0.11.0"
lazy_static     = "1.4.0"
serde = "1.0.114"
serde_json = "1.0.69"
static_assertions = "1.1.0"
spl-token = {{ version = "3.0.0-pre1", features = ["no-entrypoint"] }}

[dependencies.massbit-solana-sdk]
package = "massbit-solana-sdk"
#git = "https://github.com/massbitprotocol/massbitprotocol.git"
#branch = "main"
path = "../../../massbitprotocol/chain/solana-sdk"

[dependencies.solana-transaction-status]
package = "solana-transaction-status"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"

[dependencies.solana-account-decoder]
package = "solana-account-decoder"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"


[dependencies.solana-client]
package = "solana-client"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"

[dependencies.solana-sdk]
package = "solana-sdk"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"

[dependencies.solana-program]
package = "solana-program"
git = "https://github.com/massbitprotocol/solana.git"
branch = "massbit"

[dev-dependencies]
tokio = "1.15.0"

[lib]
crate-type = ["cdylib", "lib"]            
"#
            );
            match std::fs::write(&output_path, &content) {
                Ok(res) => {
                    log::info!("Write file cargo.toml success full");
                }
                Err(err) => {
                    log::error!("{:?}", &err);
                }
            }
        }
    }
    fn gen_lib(&self) {
        let output_path = format!("{}/src/lib.rs", &self.config.output_logic);
        let content = format!(
            r#"
pub mod generated;
pub mod mapping;

use lazy_static::lazy_static;
use massbit_solana_sdk::{{export_plugin, plugin::{{handler::SolanaHandler, PluginRegistrar}}, store::IndexStore, types::SolanaBlock}};
use solana_client::rpc_client::RpcClient;
use std::env;
use std::error::Error;
use std::sync::Arc;
use libloading::Library;
use massbit_solana_sdk::smart_contract::SmartContractProxy;
use massbit_solana_sdk::smart_contract::{{InstructionInterface, InstructionParser, SmartContractRegistrar}};
use massbit_solana_sdk::transport::interface::InterfaceRegistrar;
lazy_static! {{
    pub static ref SOLANA_CLIENT: Arc<RpcClient> = Arc::new(RpcClient::new(
        env::var("SOLANA_RPC_URL").unwrap_or(String::from("http://194.163.156.242:8899"))
    ));
}}
pub const ADDRESS: &str = "{program_id}";

#[doc(hidden)]
#[no_mangle]
pub static mut STORE: Option<&mut dyn IndexStore> = None;
#[no_mangle]
pub static mut INTERFACE: Option<&mut dyn InstructionParser> = None;
export_plugin!(register);

#[allow(dead_code, improper_ctypes_definitions)]
extern "C" fn register(registrar: &mut dyn PluginRegistrar) {{
    registrar.register_solana_handler(Box::new(SolanaHandlerAdapter));
}}

#[derive(Clone)]
pub struct SolanaHandlerAdapter;

impl SolanaHandler for SolanaHandlerAdapter {{
     fn handle_blocks(&self, blocks: &Vec<SolanaBlock>) -> Result<i64, Box<dyn Error>> {{
        println!("Start handle_blocks, block len: {{}}", blocks.len());
        let mut block_slot = -1_i64;
        // Todo: Rewrite the flush so it will flush after finish the array of blocks for better performance. For now, we flush after each block.
         unsafe {{
             if let Some(interface) = INTERFACE.as_mut() {{
                 for block in blocks {{
                     mapping::handle_block(*interface, block);
                     block_slot = block_slot.max(block.block_number as i64);
                     if let Some(store) = &mut STORE {{
                         store.flush(&block.block.blockhash, block.block_number);
                     }}
                 }}
             }}
         }}
        Ok(block_slot)
    }}
}}"#,
            program_id = &self.config.contract_address
        );
        match std::fs::write(&output_path, &content) {
            Ok(_) => {
                use std::process::Command;
                let _ = Command::new("rustfmt").arg(output_path).output();
                log::info!("Write file cargo.toml success full");
            }
            Err(err) => {
                log::error!("{:?}", &err);
            }
        }
    }
}
