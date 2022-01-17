use crate::config::IndexerConfig;
use crate::parser::visittor::Visitor;
use syn::__private::ToTokens;
use syn::{Attribute, File, Item, ItemEnum, ItemUse, Variant};

#[derive(Default)]
pub struct InstructionHandler {
    pub config: IndexerConfig,
}

impl InstructionHandler {
    pub fn new(config: IndexerConfig) -> Self {
        Self { config }
    }
}
impl Visitor for InstructionHandler {
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        //println!("{:?}", item_enum.to_token_stream().to_string());
        let ident = item_enum.ident.to_string();
        if self.config.main_instruction.as_str() == ident.as_str() {
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
    }

    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) {
        println!("Enum name {:?}", variant);
    }

    fn visit_item_use(&mut self, item_use: &ItemUse) {}
}
