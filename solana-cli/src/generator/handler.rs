use crate::generator::graphql::{MAPPING_DB_TYPES_TO_RUST, MAPPING_RUST_TYPES_TO_DB};
use crate::generator::Generator;
use crate::schema::{Schema, Variant, VariantArray};
use inflector::Inflector;
use std::fmt::Write;

const MODULES: &str = r#"
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
const ENTITY_SAVE: &str = r#"
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

impl<'a> Generator<'a> {
    pub fn generate_handler(&self, schema: &Schema) -> String {
        let mut out = String::new();
        writeln!(out, "{}", MODULES);
        writeln!(out, "{}", ENTITY_SAVE);
        if schema.name.is_some() && schema.variants.is_some() {
            let name = schema.get_pascal_name(schema.name.as_ref().unwrap());
            let patterns = self.expand_handler_patterns(&name, schema.variants.as_ref().unwrap());
            let handler_functions =
                self.expand_handler_functions(&name, schema.variants.as_ref().unwrap());
            write!(
                &mut out,
                r#"pub struct Handler {{}}
                    impl Handler {{
                        pub fn process(
                            &self, 
                            block: &SolanaBlock, 
                            transaction: &TransactionWithStatusMeta, 
                            program_id: &Pubkey, 
                            accounts: &Vec<Pubkey>, 
                            input: &[u8],
                        ) {{
                            println!("Process block {{}} with input {{:?}}", block.block_number, input);
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
                let args = match &variant.inner_type {
                    None => (String::default(), String::default()),
                    Some(_) => (String::from("(arg)"), String::from(", arg")),
                };
                format!(
                    r#"{enum_name}::{var_name}{var_inner} => {{
                        self.{method_name}(block,transaction,program_id, accounts{arg});
                    }}"#,
                    enum_name = enum_name,
                    var_name = &variant.name,
                    method_name = method_name,
                    var_inner = &args.0,
                    arg = &args.1
                )
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
                let function_body = self.expand_function_body(variant);
                let mut inner_arg = String::default();
                if let Some(inner_type) = &variant.inner_type {
                    write!(&mut inner_arg, "arg: {}", inner_type);
                };
                let log = if let Some(inner_type) = &variant.inner_type {
                    format!(r#"println!("call function {} for handle incoming block {{}} with argument {{:?}}", block.block_number, &arg);"#, function_name)
                } else {
                    format!(r#"println!("call function {} for handle incoming block {{}}", block.block_number);"#, function_name)
                };
                format!(
                    r#"pub fn {function_name}(
                                &self,
                                block: &SolanaBlock,
                                transaction: &TransactionWithStatusMeta,
                                program_id: &Pubkey,
                                accounts: &Vec<Pubkey>,
                                {inner_arg}
                            ) -> Result<(), anyhow::Error> {{
                                {log}
                                {function_body}
                            }}"#,
                    function_name = function_name,
                    log = log,
                    function_body = function_body,
                    inner_arg = inner_arg
                )
            })
            .collect::<Vec<String>>()
    }
    pub fn expand_function_body(&self, variant: &Variant) -> String {
        let mut assignments: Vec<String> = Vec::default();
        //Account assigment
        if let Some(accounts) = &variant.accounts {
            for account in accounts {
                assignments.push(format!(
                    r#"map.insert("{}".to_string(), Value::try_from(
                        accounts.get({})
                            .and_then(|pubkey| Some(pubkey.to_string()))
                            .unwrap_or_default()));"#,
                    account.name, account.index
                ));
            }
        }
        // Write table if there is inner_type
        if let Some(inner_type) = &variant.inner_type {
            // Get definitions
            if let Some(inner_schema) = self.definitions.get(inner_type.as_str()) {
                // get a table corresponding to inner_schema
                self.expand_entity_assignment(&mut assignments, variant, inner_schema);
            } else if MAPPING_RUST_TYPES_TO_DB.contains_key(inner_type.as_str()) {
                //Inner type is primitive
                self.expand_single_assignment(&mut assignments, "value", "arg");
            }
        }
        format!(
            r#"
                let mut map : HashMap<Attribute, Value> = HashMap::default();
                map.insert("id".to_string(), Value::try_from(Uuid::new_v4().to_simple().to_string()));
                {assignments}
                Entity::from(map).save("{entity_name}");
                Ok(())
            "#,
            assignments = assignments.join("\n"),
            entity_name = &variant.name
        )
    }
    fn expand_single_assignment(
        &self,
        assignments: &mut Vec<String>,
        field_name: &str,
        field_value: &str,
    ) {
        assignments.push(format!(
            r#"map.insert("{}".to_string(), Value::try_from({}));"#,
            field_name, field_value
        ));
    }
    pub fn expand_entity_assignment(
        &self,
        assignments: &mut Vec<String>,
        variant: &Variant,
        inner_schema: &Schema,
    ) {
        //If inner schema is a struct
        if let Some(properties) = &inner_schema.properties {
            for property in properties {
                let db_type = MAPPING_RUST_TYPES_TO_DB.get(property.data_type.as_str());
                //.unwrap_or(&*DEFAULT_TYPE_DB);
                // If data_type is not primitive (e.g. Enum, Struct)
                match db_type {
                    // If data_type is primitive (e.g. Enum, Struct)
                    Some(db_type) => {
                        let elm_value = if property.data_type.starts_with("NonZero") {
                            format!("{}.get()", property.name)
                        } else {
                            format!("{}", property.name)
                        };
                        let property_value = match property.array_length {
                            Some(_) => {
                                format!(
                                    r#"arg.{property_name}.iter().map(|&{property_name}| Value::try_from({elm_value})).collect::<Vec<Value>>()"#,
                                    property_name = &property.name,
                                    elm_value = elm_value
                                )
                            }
                            None => {
                                format!("arg.{}", elm_value)
                            }
                        };
                        assignments.push(format!(
                            r#"map.insert("{}".to_string(), Value::try_from({}));"#,
                            &property.name, &property_value
                        ));
                    }
                    None => {
                        assignments.push(format!(
                            r#"map.insert("{name}".to_string(), Value::try_from(serde_json::to_string(&arg.{name}).unwrap_or(Default::default())));"#,
                            name=&property.name
                        ));
                    }
                }
            }
        }
    }
}
