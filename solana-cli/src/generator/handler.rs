use crate::generator::graphql::{
    DEFAULT_TYPE_DB, MAPPING_DB_TYPES_TO_RUST, MAPPING_RUST_TYPES_TO_DB,
};
use crate::schema::{PropertyArray, Schema, Variant, VariantArray};
use inflector::Inflector;
use std::fmt::Write;

const modules: &str = r#"
use crate::generated::instruction::*;
use crate::models::*;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use uuid::Uuid;
use serde_json;
use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
"#;
pub fn append_handler_modules(out: &mut String) {
    writeln!(out, "{}", modules);
}
impl Schema {
    pub fn gen_handler(&self) -> String {
        let mut out = String::new();
        writeln!(out, "{}", modules);
        if self.name.is_some() && self.variants.is_some() {
            let name = self.get_pascal_name(self.name.as_ref().unwrap());
            let patterns = self.expand_handler_patterns(&name, self.variants.as_ref().unwrap());
            let handler_functions =
                self.expand_handler_functions(&name, self.variants.as_ref().unwrap());
            write!(
                &mut out,
                r#"
pub struct Handler {{}}
    impl Handler {{
        pub fn process(&self, block: &SolanaBlock, transaction: &SolanaTransaction, program_id: &Pubkey, accounts: &[Account], input: &[u8]) {{
            if let Some(instruction) = {name}::unpack(input) {{
                match instruction {{
                    {patterns}
                }}
            }}
        }}
        {handler_functions}
    }}"#,
                name = name,
                patterns = patterns.join(",\n"),
                handler_functions = handler_functions.join("\n")
            );
        }
        out
    }
    pub fn expand_handler_patterns(
        &self,
        enum_name: &String,
        variants: &VariantArray,
    ) -> Vec<String> {
        variants
            .iter()
            .map(|variant| {
                let method_name = format!("process_{}", &variant.name.to_snake_case());
                match &variant.inner_type {
                    None => {
                        format!(
                            r#"
                    {enum_name}::{var_name} => {{
                        self.{method_name}(block,transaction,program_id, accounts);
                    }}"#,
                            enum_name = enum_name,
                            var_name = &variant.name,
                            method_name = method_name
                        )
                    }
                    Some(inner_type) => {
                        format!(
                            r#"
                    {enum_name}::{var_name}(arg) => {{
                        self.{method_name}(block,transaction,program_id, accounts, arg);
                    }}"#,
                            enum_name = enum_name,
                            var_name = &variant.name,
                            method_name = method_name
                        )
                    }
                }
            })
            .collect::<Vec<String>>()
    }
    pub fn expand_handler_functions(
        &self,
        enum_name: &String,
        variants: &VariantArray,
    ) -> Vec<String> {
        variants
            .iter()
            .map(|variant| {
                let function_name = format!("process_{}", &variant.name.to_snake_case());
                let function_body = self.gen_function_body(variant);

                match &variant.inner_type {
                    None => {
                        format!(
                            r#"pub fn {function_name}(
                                    &self,
                                    program_id: &Pubkey,
                                    accounts: &[Account],
                                ) -> Result<(), anyhow::Error> {{
                                    {function_body}
                                }}"#,
                            function_name = function_name,
                            function_body = function_body,
                        )
                    }
                    Some(inner_type) => {
                        format!(
                            r#"
        pub fn {function_name}(
                                &self,
                                block: &SolanaBlock,
                                transaction: &SolanaTransaction,
                                program_id: &Pubkey,
                                accounts: &[Account],
                                arg: {inner_type}
                            ) -> Result<(), anyhow::Error> {{
                                {function_body}
                        
        }}"#,
                            function_name = function_name,
                            function_body = function_body,
                            inner_type = inner_type
                        )
                    }
                }
            })
            .collect::<Vec<String>>()
    }
    pub fn gen_function_body(&self, variant: &Variant) -> String {
        let mut out = String::new();

        // Write table if there is inner_type
        if let Some(inner_type) = variant.inner_type.clone() {
            // Get definitions
            if let Some(sub_schema) = self.definitions.get(&inner_type) {
                // get a table corresponding to sub_schema
                let str_entity: String =
                    Schema::gen_entity_assignment(sub_schema, &inner_type, &variant.name);
                write!(out, "{}", str_entity);
            } else if MAPPING_RUST_TYPES_TO_DB.contains_key(inner_type.as_str()) {
                let str_entity: String = Schema::gen_entity_assignment(
                    &Schema::default(),
                    &inner_type,
                    &variant.name.clone(),
                );
                write!(out, "{}", str_entity);
            }
        }
        // Tail of function
        write!(
            out,
            r#"
            Ok(())"#
        );
        out
    }
    pub fn gen_entity_assignment(
        schema: &Schema,
        entity_type: &String,
        entity_name: &String,
    ) -> String {
        let mut out = String::new();

        write!(
            out,
            r#"
            let id = Uuid::new_v4().to_simple().to_string();
            let entity = {} {{
                id,"#,
            entity_name
        );
        match MAPPING_RUST_TYPES_TO_DB.get(entity_type.as_str()) {
            // if it is primitive type
            Some(db_type) => {
                write!(
                    out,
                    r#"
    value: arg as {},"#,
                    MAPPING_DB_TYPES_TO_RUST
                        .get(db_type)
                        .unwrap_or(&Default::default())
                );
            }
            // if it is not primitive type
            None => {
                if let Some(properties) = &schema.properties {
                    for property in properties {
                        let db_type = MAPPING_RUST_TYPES_TO_DB.get(property.data_type.as_str());
                        //.unwrap_or(&*DEFAULT_TYPE_DB);
                        // If data_type is not primitive (e.g. Enum, Struct)
                        match db_type {
                            // If data_type is primitive (e.g. Enum, Struct)
                            Some(db_type) => {
                                write!(
                                    out,
                                    r#"
                {}: arg.{} as {},"#,
                                    property.name,
                                    property.name,
                                    MAPPING_DB_TYPES_TO_RUST
                                        .get(db_type)
                                        .unwrap_or(&Default::default())
                                );
                            }
                            None => {
                                write!(
                                    out,
                                    r#"
                {}: serde_json::to_string(arg.{}),"#,
                                    property.name, property.name,
                                );
                            }
                        }
                    }
                }
            }
        };
        write!(
            out,
            r#"
            }};
            entity.save();"#
        );
        out
    }
}
