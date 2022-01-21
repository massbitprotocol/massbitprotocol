use super::{Definitions, InstructionHandler, InstructionParser, Visitor};
use crate::config::IndexerConfig;
use crate::parser::instruction_meta::InstructionMeta;
use crate::parser::schema::GraphqlSchema;
use crate::schema;
use crate::schema::{AccountInfo, InstructionSchema, Schema, VariantArray};
use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};
use syn::__private::ToTokens;
use syn::{
    Attribute, Field, Fields, FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Type,
    Variant,
};

pub struct IndexerBuilder<'a> {
    pub config_path: &'a str,
    config: Option<IndexerConfig>,
    gen_meta: bool,
    context: VisitorContext<'a>,
}
impl<'a> IndexerBuilder<'a> {
    fn default() -> Self {
        Self {
            config_path: "",
            config: None,
            gen_meta: false,
            context: VisitorContext::default(),
        }
    }
    pub fn builder() -> IndexerBuilder<'a> {
        IndexerBuilder::default()
    }
    pub fn with_config_path(mut self, path: &'a str) -> Self {
        self.config_path = path;
        let json = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("Unable to read `{}`: {}", path, err));
        let config: IndexerConfig = serde_json::from_str(&json)
            .unwrap_or_else(|err| panic!("Cannot parse `{}` as JSON: {}", path, err));
        self.config = Some(config);
        self
    }
    pub fn gen_meta(mut self, gen_meta: bool) -> Self {
        self.gen_meta = gen_meta;
        self
    }
    pub fn build(&mut self) {
        let config_parser = self.config.as_ref().map(|config| {
            let mut parts = config
                .main_instruction
                .split("::")
                .into_iter()
                .map(|part| String::from(part))
                .collect::<Vec<String>>();
            let instruction_name = parts.remove(parts.len() - 1);
            let config = self.config.as_ref().unwrap().clone();
            println!("Parse definitions");
            let mut definitions = Definitions::new(config.clone());
            definitions.current_paths = parts.clone();
            definitions.visit_module(config.smart_contract_source.as_str(), &parts);
            let instruction_path = format!(
                "{}/src/{}.rs",
                config.smart_contract_source,
                parts.join("/")
            );
            log::info!(
                "Load instruction {:?} from source file: {:?}",
                &instruction_name,
                &instruction_path
            );
            let input_content =
                fs::read_to_string(instruction_path.as_str()).unwrap_or_else(|_| {
                    panic!(
                        "Something went wrong reading the file {}",
                        &instruction_path
                    )
                });
            (instruction_name, input_content, definitions)
        });
        if let Some((main_instruction, content, definitions)) = config_parser {
            self.config.as_mut().unwrap().main_instruction = main_instruction;
            if let Ok(ast) = syn::parse_file(&content) {
                let config = self.config.as_ref().unwrap().clone();
                //Gen instruction file in ouput
                let file_meta = "instruction.json";
                let config_path = Path::new(self.config_path)
                    .parent()
                    .map(|path| format!("{}", path.to_string_lossy()))
                    .unwrap_or_default();
                let file_path = format!("{}/{}", &config_path, &file_meta);
                if !Path::new(file_path.as_str()).exists() {
                    let mut instruction_meta = InstructionMeta::new(config_path, config.clone());
                    instruction_meta.visit_file(&ast);
                    instruction_meta.write_output(file_meta);
                }
                let json = std::fs::read_to_string(file_path.as_str())
                    .unwrap_or_else(|err| panic!("Unable to read `{}`: {}", &file_path, err));
                let variant_accounts: HashMap<String, Vec<AccountInfo>> =
                    serde_json::from_str(&json).unwrap_or_else(|err| {
                        panic!("Cannot parse `{}` as JSON: {}", &file_path, err)
                    });

                println!("Create Handler");
                let mut handler =
                    InstructionHandler::new(config.clone(), &definitions, &variant_accounts);
                handler.visit_file(&ast);
                handler.write_output("handler.rs");
                let mut parser =
                    InstructionParser::new(config.clone(), &definitions, &variant_accounts);
                parser.visit_file(&ast);
                parser.write_output("instruction.rs");
                let mut graphql = GraphqlSchema::new(config, &definitions, &variant_accounts);
                graphql.visit_file(&ast);
                graphql.write_output("schema.graphql");
            };
        }
    }
    pub fn write_to_file<P: ?Sized + AsRef<Path>>(
        &self,
        output_path: &P,
        content: &str,
    ) -> io::Result<()> {
        match fs::write(output_path, content) {
            Ok(_) => {
                log::info!(
                    "Write content to file {:?} successfully",
                    &output_path.as_ref().as_os_str()
                );
                Ok(())
            }
            e @ Err(_) => {
                log::info!(
                    "Write content to file {:?} fail. {:?}",
                    &output_path.as_ref().as_os_str(),
                    &e
                );
                e
            }
        }
    }
}
#[derive(Default)]
pub struct VisitorContext<'a> {
    current_item: Option<&'a Item>,
}
impl<'a> IndexerBuilder<'a> {
    fn create_variant(&self, item_variant: &Variant) -> schema::Variant {
        let mut variant = schema::Variant {
            name: item_variant.ident.to_string(),
            value: None,
            inner_name: None,
            inner_type: None,
            inner_scope: None,
            description: None,
            offset: None,
            variant_tag: 0,
            accounts: None,
        };
        for field in item_variant.fields.iter() {
            match &field.ty {
                Type::Array(_) => {}
                Type::BareFn(_) => {}
                Type::Group(_) => {}
                Type::ImplTrait(_) => {}
                Type::Infer(_) => {}
                Type::Macro(_) => {}
                Type::Never(_) => {}
                Type::Paren(_) => {}
                Type::Path(path) => {
                    variant.inner_type =
                        path.path.segments.first().map(|seg| seg.ident.to_string());
                }
                Type::Ptr(_) => {}
                Type::Reference(_) => {}
                Type::Slice(_) => {}
                Type::TraitObject(_) => {}
                Type::Tuple(_) => {}
                Type::Verbatim(_) => {}
                Type::__TestExhaustive(_) => {}
            }
        }

        variant
    }
}
/*
#[doc = " Instructions supported by the Fraction program."]
impl<'a> Visitor for IndexerBuilder<'a> {
    #[doc = " Instructions supported by the Fraction program."]
    fn visit_file(&mut self, file: &File) {
        for item in file.items.iter() {
            self.visit_item(item);
        }
        for attr in file.attrs.iter() {
            self.visit_attribute(attr);
        }
    }

    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Const(_) => {}
            Item::Enum(item_enum) => {
                self.visit_item_enum(item_enum);
            }
            Item::ExternCrate(_) => {}
            Item::Fn(_) => {}
            Item::ForeignMod(_) => {}
            Item::Impl(_) => {}
            Item::Macro(_) => {}
            Item::Macro2(_) => {}
            Item::Mod(_) => {}
            Item::Static(_) => {}
            Item::Struct(_) => {}
            Item::Trait(_) => {}
            Item::TraitAlias(_) => {}
            Item::Type(_) => {}
            Item::Union(_) => {}
            Item::Use(item_use) => {
                self.visit_item_use(item_use);
            }
            Item::Verbatim(_) => {}
            Item::__TestExhaustive(_) => {}
        }
    }

    fn visit_attribute(&mut self, attribute: &Attribute) {
        todo!()
    }

    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        println!("{:?}", item_enum.to_token_stream().to_string());
        let ident = item_enum.ident.to_string();
        // if self.enums.contains(&ident) && self.enums.len() == 1 {
        //     self.schema.name = Some(ident.clone());
        //     self.schema.variants = Some(Vec::new());
        // }
        for attr in item_enum.attrs.iter() {
            println!("{:?}", attr);
            println!("{:?}", attr.to_token_stream().to_string());
        }
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
    fn visit_item_use(&mut self, item_use: &ItemUse) {}

    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {}

    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {}

    fn visit_unit_field(&mut self, ident_name: &String) {}

    fn create_content(&self) -> String {
        String::new()
    }

    fn create_dir_path(&self) -> String {
        String::new()
    }
}
*/
