use crate::config::IndexerConfig;
use crate::parser::visitor::Visitor;
use syn::__private::ToTokens;
use syn::{Attribute, Fields, FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Variant};

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

    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) {
        let ident_name = variant.ident.to_string();
        match &variant.fields {
            Fields::Named(named_field) => {
                self.visit_named_field(&ident_name, named_field);
            }
            Fields::Unnamed(fields_unnamed) => {
                self.visit_unnamed_field(&ident_name, fields_unnamed);
            }
            Fields::Unit => self.visit_unit_field(&ident_name),
        }
        //println!("Enum name {:?}", variant);
    }

    fn visit_item_use(&mut self, item_use: &ItemUse) {}
    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {
        println!("Named fields {:?}", field_named);
    }
    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {
        println!(
            "Ident: {:?}, Unnamed fields {:?}",
            ident_name,
            field_unnamed
                .unnamed
                .iter()
                .map(|field| field.ty.to_token_stream().to_string())
                .collect::<Vec<String>>()
        );
    }
    fn visit_unit_field(&mut self, ident_name: &String) {}
}
