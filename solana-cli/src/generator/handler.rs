use crate::generator::graphql::{
    DEFAULT_TYPE_DB, MAPPING_DB_TYPES_TO_RUST, MAPPING_RUST_TYPES_TO_DB,
};
use crate::schema::{PropertyArray, Schema, Variant, VariantArray};
use inflector::Inflector;
use std::fmt::Write;

const modules: &str = r#"
use crate::generated::instruction::*;
use crate::STORE;
use crate::{Attribute, Entity, EntityFilter, EntityOrder, EntityRange, Value};
//use crate::models::*;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use uuid::Uuid;
use serde_json;
use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use solana_transaction_status::{parse_instruction, ConfirmedBlock, TransactionWithStatusMeta};
use std::collections::HashMap;
"#;
const entity_save: &str = r#"
pub trait EntityExt {
    fn save(&self, entity_name: &str);
}
impl EntityExt for Entity {
    fn save(&self, entity_name: &str) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save(String::from(entity_name), self.clone());
        }
    }
}
pub trait ValueExt<T>: Sized {
    fn try_from(_: T) -> Self;
}    
impl ValueExt<String> for Value {
    fn try_from(value: String) -> Value {
        Value::String(value)
    }
}
impl ValueExt<u8> for Value {
    fn try_from(value: u8) -> Value {
        Value::Int(value as i32)
    }
}
impl ValueExt<i8> for Value {
    fn try_from(value: i8) -> Value {
        Value::Int(value as i32)
    }
}
impl ValueExt<u16> for Value {
    fn try_from(value: u16) -> Value {
        Value::Int(value as i32)
    }
}
impl ValueExt<i16> for Value {
    fn try_from(value: i16) -> Value {
        Value::Int(value as i32)
    }
}
impl ValueExt<u32> for Value {
    fn try_from(value: u32) -> Value {
        Value::Int(value as i32)
    }
}
impl ValueExt<i32> for Value {
    fn try_from(value: i32) -> Value {
        Value::Int(value)
    }
}
impl ValueExt<u64> for Value {
    fn try_from(value: u64) -> Value {
        Value::BigInt(value.into())
    }
}
impl ValueExt<i64> for Value {
    fn try_from(value: i64) -> Value {
        Value::BigInt(value.into())
    }
}
impl ValueExt<Vec<Value>> for Value {
    fn try_from(value: Vec<Value>) -> Value {
        Value::List(value)
    }
}
"#;

impl Schema {
    pub fn gen_handler(&self) -> String {
        let mut out = String::new();
        writeln!(out, "{}", modules);
        writeln!(out, "{}", entity_save);
        if self.name.is_some() && self.variants.is_some() {
            let name = self.get_pascal_name(self.name.as_ref().unwrap());
            let patterns = self.expand_handler_patterns(&name, self.variants.as_ref().unwrap());
            let handler_functions =
                self.expand_handler_functions(&name, self.variants.as_ref().unwrap());
            write!(
                &mut out,
                r#"pub struct Handler {{}}
                    impl Handler {{
                        pub fn process(&self, block: &SolanaBlock, transaction: &TransactionWithStatusMeta, program_id: &Pubkey, accounts: &[Account], input: &[u8]) {{
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
                            r#"{enum_name}::{var_name} => {{
                                self.{method_name}(block,transaction,program_id, accounts);
                            }}"#,
                            enum_name = enum_name,
                            var_name = &variant.name,
                            method_name = method_name
                        )
                    }
                    Some(inner_type) => {
                        format!(
                            r#"{enum_name}::{var_name}(arg) => {{
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
                                    block: &SolanaBlock,
                                    transaction: &TransactionWithStatusMeta,
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
                            r#"pub fn {function_name}(
                                &self,
                                block: &SolanaBlock,
                                transaction: &TransactionWithStatusMeta,
                                program_id: &Pubkey,
                                accounts: &[Account],
                                arg: {inner_type}
                            ) -> Result<(), anyhow::Error> {{
                                println!("{{:?}}", &arg);
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
        let mut attributes: Vec<String> = Vec::default();
        match MAPPING_RUST_TYPES_TO_DB.get(entity_type.as_str()) {
            // if it is primitive type
            Some(db_type) => {
                attributes.push(format!(
                    r#"map.insert("value".to_string(), Value::try_from(arg));"#,
                    // MAPPING_DB_TYPES_TO_RUST
                    //     .get(db_type)
                    //     .unwrap_or(&Default::default())
                ));
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
                                let property_name = if property.data_type.starts_with("NonZero") {
                                    format!("{}.get()", property.name)
                                } else {
                                    format!("{}", property.name)
                                };

                                match property.array_length {
                                    Some(array_length) => {
                                        // Todo: this code is tricky, should revise.
                                        attributes.push(format!(
                                            r#"map.insert("{}".to_string(), Value::try_from(arg.{}.iter().map(|&{}| Value::try_from({})).collect::<Vec<Value>>()));"#,
                                            property.name,
                                            property.name,
                                            property.name,
                                            property_name,
                                            // MAPPING_DB_TYPES_TO_RUST
                                            //     .get(db_type)
                                            //     .unwrap_or(&Default::default())
                                        ));
                                    }
                                    None => {
                                        attributes.push(format!(
                                            r#"map.insert("{}".to_string(), Value::try_from(arg.{}));"#,
                                            property.name,
                                            property_name,
                                            // MAPPING_DB_TYPES_TO_RUST
                                            //     .get(db_type)
                                            //     .unwrap_or(&Default::default())
                                        ));
                                    }
                                }
                            }
                            None => {
                                attributes.push(format!(
                                    r#"map.insert("{name}".to_string(), Value::try_from(serde_json::to_string(&arg.{name}).unwrap_or(Default::default())));"#,
                                    name=property.name
                                ));
                            }
                        }
                    }
                }
            }
        };
        format!(
            r#"
        let mut map : HashMap<Attribute, Value> = HashMap::default();
        map.insert("id".to_string(), Value::try_from(Uuid::new_v4().to_simple().to_string()));
        {attributes}
        Entity::from(map).save("{entity_name}");
        "#,
            attributes = attributes.join("\n"),
            entity_name = entity_name
        )
    }

    pub fn gen_entity_assignment0(
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
                    r#"value: arg as {},"#,
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
                                let property_name = match property.data_type.starts_with("NonZero")
                                {
                                    true => {
                                        format!("{}.get()", property.name)
                                    }
                                    false => {
                                        format!("{}", property.name)
                                    }
                                };

                                match property.array_length {
                                    Some(array_length) => {
                                        // Todo: this code is tricky, should revise.
                                        write!(
                                            out,
                                            r#"{}: arg.{}.iter().map(|&{}| {} as {}).collect(),"#,
                                            property.name,
                                            property.name,
                                            property.name,
                                            property_name,
                                            MAPPING_DB_TYPES_TO_RUST
                                                .get(db_type)
                                                .unwrap_or(&Default::default())
                                        );
                                    }
                                    None => {
                                        write!(
                                            out,
                                            r#"{}: arg.{} as {},"#,
                                            property.name,
                                            property_name,
                                            MAPPING_DB_TYPES_TO_RUST
                                                .get(db_type)
                                                .unwrap_or(&Default::default())
                                        );
                                    }
                                }
                            }
                            None => {
                                write!(
                                    out,
                                    r#"{}: serde_json::to_string(&arg.{}).unwrap_or(Default::default()),"#,
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
