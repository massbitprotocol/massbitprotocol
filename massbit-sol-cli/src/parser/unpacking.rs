use super::Visitor;
use crate::config::IndexerConfig;
use crate::parser::Definitions;
use crate::schema::AccountInfo;
use inflector::Inflector;
use std::collections::HashMap;
use std::fmt::Write;
use syn::__private::ToTokens;
use syn::{FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Variant};

const MODULES: &str = r#"
use std::error::Error;
use mpl_metaplex::instruction::MetaplexInstruction;
use borsh::BorshDeserialize;
use transport::interface::InstructionParser as InstructionParserTrait;
use transport::{TransportValue, Value};
"#;

pub struct InstructionParser<'a> {
    pub config: IndexerConfig,
    definitions: &'a Definitions,
    patterns: Vec<String>,
    unpack_functions: Vec<String>,
    variant_accounts: &'a HashMap<String, Vec<AccountInfo>>,
}

impl<'a> InstructionParser<'a> {
    pub fn new(
        config: IndexerConfig,
        definitions: &'a Definitions,
        variant_accounts: &'a HashMap<String, Vec<AccountInfo>>,
    ) -> Self {
        Self {
            config,
            definitions,
            patterns: vec![],
            unpack_functions: vec![],
            variant_accounts,
        }
    }
}

impl<'a> Visitor for InstructionParser<'a> {
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        let ident = item_enum.ident.to_string();
        if self.config.main_instruction.as_str() == ident.as_str() {
            // for attr in item_enum.attrs.iter() {
            //     println!("{:?}", attr);
            //     println!("{:?}", attr.to_token_stream().to_string());
            // }
            item_enum
                .variants
                .iter()
                .for_each(|variant| self.visit_item_variant(item_enum, variant));
        }
    }

    fn visit_item_use(&mut self, item_use: &ItemUse) {}

    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {
        let ident_snake = ident_name.to_snake_case();

        let inner_names = field_named
            .named
            .iter()
            .map(|field| field.ty.to_token_stream().to_string())
            .collect::<Vec<String>>();
        if inner_names.len() == 1 {
            let output = format!(
                r#"{}::{}(input) => {{
                let mut transport_value = self.unpack_{}(input);
                transport_value
            }}"#,
                self.config.main_instruction, ident_name, &ident_snake
            );
            self.patterns.push(output);
            //Generate process function
            let inner_type = field_named
                .named
                .iter()
                .last()
                .unwrap()
                .ty
                .to_token_stream()
                .to_string();
            let item_def = self.definitions.get_item_def(&inner_type);
            if let Some(struct_def) = item_def {
                let function = format!(
                    r#"fn unpack_{fn_name}(
                        &self,
                        input: {inner_type},
                    ) -> Result<TransportValue, anyhow::Error> {{
                        {body}
                    }}"#,
                    fn_name = ident_snake,
                    inner_type = format!(
                        "{}::{}",
                        struct_def.get_module_path(),
                        inner_names.get(0).unwrap(),
                    ),
                    body = struct_def.create_unpack_function(&ident_name, self.definitions),
                );
                self.unpack_functions.push(function);
            }
        }
    }
    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {
        let ident_snake = &ident_name.to_snake_case();
        //Todo: handle case field_unnamed has only one inner (field_unnamed.len() == 1)
        let inner_names = field_unnamed
            .unnamed
            .iter()
            .map(|field| field.ty.to_token_stream().to_string())
            .collect::<Vec<String>>();
        if inner_names.len() == 1 {
            let output = format!(
                r#"{}::{}(input) => {{self.unpack_{}(input)}}"#,
                self.config.main_instruction, ident_name, &ident_snake
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
            let item_struct = self.definitions.get_item_def(&inner_type);
            if let Some(struct_def) = item_struct {
                let function = format!(
                    r#"fn unpack_{fn_name}(
                        &self,
                        input: {inner_type},
                    ) -> Result<TransportValue, anyhow::Error> {{
                        {body}
                    }}"#,
                    fn_name = ident_snake,
                    inner_type = format!(
                        "{}::{}",
                        struct_def.get_module_path(),
                        inner_names.get(0).unwrap()
                    ),
                    body = struct_def.create_unpack_function(&ident_name, self.definitions),
                );
                self.unpack_functions.push(function);
            }
        } else if field_unnamed.unnamed.len() > 1 {
        }
    }
    fn visit_unit_field(&mut self, ident_name: &String) {
        let ident_snake = ident_name.to_snake_case();
        let output = format!(
            r#"{}::{} => {{self.unpack_{}()}}"#,
            self.config.main_instruction, ident_name, &ident_snake
        );
        self.patterns.push(output);
        //Generate process function
        let function = format!(
            r#"fn unpack_{fn_name}(
                &self
            ) -> Result<TransportValue, anyhow::Error> {{
                let mut transport_value = TransportValue::new("{entity_name}");
                Ok(transport_value)
            }}"#,
            entity_name = ident_name,
            fn_name = ident_snake
        );
        self.unpack_functions.push(function);
    }

    fn create_content(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(&mut out, "{}", MODULES);
        let _ = writeln!(
            &mut out,
            r#"#[derive(Debug, Clone, PartialEq)]
                pub struct InstructionParser;
                impl InstructionParserTrait for InstructionParser {{
                    fn unpack_instruction(&self, input: &[u8]) -> Result<TransportValue, anyhow::Error> {{
                        let instruction = {instruction}::{fn_unpacking}(input)?;                                     
                        match instruction {{
                            {patterns}
                        }}
                    }}                  
                }}
                impl InstructionParser {{
                    {unpack_functions}
                }}"#,
            instruction = self.config.main_instruction,
            fn_unpacking = self.config.unpack_function,
            patterns = self.patterns.join("\n"),
            unpack_functions = self.unpack_functions.join("\n")
        );
        out
    }

    fn create_dir_path(&self) -> String {
        format!("{}/src", self.config.output_unpacking)
    }
}
