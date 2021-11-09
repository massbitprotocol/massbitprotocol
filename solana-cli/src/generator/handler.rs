use crate::schema::{PropertyArray, Schema, Variant, VariantArray};
use inflector::Inflector;
use std::fmt::Write;

const modules: &str = r#"
use crate::generated::instruction::*;
use crate::models::*;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use uuid::Uuid;
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
                r#"pub struct Handler {{}}
                impl Handler {{
                    pub fn process(&self, program_id: &Pubkey, accounts: &[Account], input: &[u8]) {{
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
                                self.{method_name}(program_id, accounts);
                            }}"#,
                            enum_name = enum_name,
                            var_name = &variant.name,
                            method_name = method_name
                        )
                    }
                    Some(inner_type) => {
                        format!(
                            r#"{enum_name}::{var_name}(arg) => {{
                                self.{method_name}(program_id, accounts, arg);
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
                            r#"pub fn {function_name}(
                                &self,
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
        let res = String::from("Ok(())");
        res
    }
}
