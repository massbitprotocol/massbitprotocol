use super::visitor::Visitor;
use super::Definitions;
use crate::config::IndexerConfig;
use inflector::Inflector;
use std::fmt::Write;
use syn::__private::ToTokens;
use syn::{Attribute, Fields, FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Variant};

const MODULES: &str = r#"
use crate::STORE;
use massbit_solana_sdk::entity::{Attribute, Entity, Value};
use massbit_solana_sdk::{
    transport::TransportValue,
    types::SolanaBlock
};
use serde_json;
use solana_program::pubkey::Pubkey;
use solana_transaction_status::TransactionWithStatusMeta;
use std::collections::HashMap;
use uuid::Uuid;
"#;
const ENTITY_SAVE: &str = r#"
pub trait TransportValueExt {
    fn save(&self);
}
impl TransportValueExt for TransportValue {
    fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save_values(&self.name, &self.values);
        }
    }
}
"#;
pub struct InstructionHandler<'a> {
    pub config: IndexerConfig,
    definitions: &'a Definitions,
    patterns: Vec<String>,
    handler_functions: Vec<String>,
}

impl<'a> InstructionHandler<'a> {
    pub fn new(config: IndexerConfig, definitions: &'a Definitions) -> Self {
        Self {
            config,
            definitions,
            patterns: vec![],
            handler_functions: vec![],
        }
    }
}
impl<'a> Visitor for InstructionHandler<'a> {
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        let ident = item_enum.ident.to_string();
        if self.config.main_instruction.as_str() == ident.as_str() {
            //Document and derive
            // for attr in item_enum.attrs.iter() {
            //     println!("{:?}", attr);
            //     println!("{:?}", attr.to_token_stream().to_string());
            // }
            println!(
                "Enum name {:?}, Variant number: {}",
                item_enum.ident,
                item_enum.variants.len()
            );
            item_enum
                .variants
                .iter()
                .for_each(|variant| self.visit_item_variant(item_enum, variant));
        }
    }

    fn visit_item_use(&mut self, item_use: &ItemUse) {}
    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {
        let ident_snake = ident_name.to_snake_case();
        let output = format!(
            r#""{}" => {{self.process_{}(block, transaction, program_id, accounts, input);}}"#,
            ident_name, &ident_snake
        );
        self.patterns.push(output);
        //Generate process function
        let function = format!(
            r#"fn process_{fn_name}(
                &self,
                block: &SolanaBlock,
                transaction: &TransactionWithStatusMeta,
                program_id: &Pubkey,
                accounts: &Vec<Pubkey>,
                input: TransportValue,
            ) -> Result<(), anyhow::Error> {{
                println!(
                    "call function process_initialize for handle incoming block {{}} with argument {{:?}}",
                    block.block_number, &input.name
                );
                input.save();
                //Entity::from(input).save("Initialize");
                println!("Write to db {{:?}}",input);
                Ok(())
            }}"#,
            fn_name = ident_snake
        );
        self.handler_functions.push(function);
    }
    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {
        let ident_snake = ident_name.to_snake_case();
        //Todo: handle case field_unnamed has only one inner (field_unnamed.len() == 1)
        if field_unnamed.unnamed.len() == 1 {
            let output = format!(
                r#""{}" => {{self.process_{}(block, transaction, program_id, accounts, input);}}"#,
                ident_name, &ident_snake
            );
            self.patterns.push(output);
            //Generate process function
            let inner_type = field_unnamed
                .unnamed
                .iter()
                .last()
                .unwrap()
                .ty
                .to_token_stream()
                .to_string();
            let item_struct = self.definitions.get_item_struct(&inner_type);
            println!("Item struct for {}: {:?}", &inner_type, item_struct);
            println!(
                "Ident: {:?}, Unnamed fields {:?}",
                ident_name,
                field_unnamed
                    .unnamed
                    .iter()
                    .map(|field| field.ty.to_token_stream().to_string())
                    .collect::<Vec<String>>()
            );
            let function = format!(
                r#"fn process_{fn_name}(
                &self,
                block: &SolanaBlock,
                transaction: &TransactionWithStatusMeta,
                program_id: &Pubkey,
                accounts: &Vec<Pubkey>,
                input: TransportValue,
            ) -> Result<(), anyhow::Error> {{
                println!(
                    "call function process_initialize for handle incoming block {{}} with argument {{:?}}",
                    block.block_number, &input.name
                );
                input.save();
                //Entity::from(input).save("Initialize");
                println!("Write to db {{:?}}",input);
                Ok(())
            }}"#,
                fn_name = ident_snake
            );
            self.handler_functions.push(function);
        } else if field_unnamed.unnamed.len() > 1 {
        }
    }
    fn visit_unit_field(&mut self, ident_name: &String) {
        let ident_snake = ident_name.to_snake_case();
        let output = format!(
            r#""{}" => {{self.process_{}(block, transaction, program_id, accounts);}}"#,
            ident_name, &ident_snake
        );
        self.patterns.push(output);
        //Generate process function
        let function = format!(
            r#"fn process_{fn_name}(
                &self,
                block: &SolanaBlock,
                transaction: &TransactionWithStatusMeta,
                program_id: &Pubkey,
                accounts: &Vec<Pubkey>
            ) -> Result<(), anyhow::Error> {{
                println!(
                    "call function process_initialize for handle incoming block {{}} without argument.",
                    block.block_number
                );
        
                //Entity::from(input).save("Initialize");
                println!("Write to db");
                Ok(())
            }}"#,
            fn_name = ident_snake
        );
        self.handler_functions.push(function);
    }
    fn create_content(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(&mut out, "{}", MODULES);
        let _ = writeln!(&mut out, "{}", ENTITY_SAVE);
        let _ = writeln!(
            &mut out,
            r#"pub struct Handler {{}}
                    impl Handler {{
                        pub fn process(
                            &self,
                            block: &SolanaBlock,
                            transaction: &TransactionWithStatusMeta,
                            program_id: &Pubkey,
                            accounts: &Vec<Pubkey>,
                            input: TransportValue,
                        ) {{
                            //println!("Process block {{}} with input {{:?}}", block.block_number, input);                           
                            match input.name.as_str() {{
                                {patterns}
                                _ => {{}}
                            }}
                            
                        }}
                        {handler_functions}
                    }}"#,
            patterns = self.patterns.join("\n"),
            handler_functions = self.handler_functions.join("\n")
        );
        out
    }
    fn create_dir_path(&self) -> String {
        format!("{}/src/generated", self.config.output_logic)
    }
}
