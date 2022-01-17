use crate::config::IndexerConfig;
use crate::parser::visittor::Visitor;
use syn::__private::ToTokens;
use syn::{Attribute, File, Item, ItemEnum, ItemUse, Variant};

#[derive(Default)]
pub struct InstructionParser {
    pub config: IndexerConfig,
}

impl InstructionParser {
    pub fn new(config: IndexerConfig) -> Self {
        Self { config }
    }
}

impl Visitor for InstructionParser {
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

    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) {}

    fn visit_item_use(&mut self, item_use: &ItemUse) {}
}
