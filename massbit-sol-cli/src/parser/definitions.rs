use crate::config::IndexerConfig;
use crate::parser::visitor::Visitor;
use syn::__private::ToTokens;
use syn::{FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Variant};

#[derive(Default)]
pub struct Definitions {
    pub config: IndexerConfig,
}

impl Definitions {
    pub fn new(config: IndexerConfig) -> Self {
        Self { config }
    }
}
impl Visitor for Definitions {
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {}

    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) {}

    fn visit_item_use(&mut self, item_use: &ItemUse) {
        println!("{:?}", &item_use.tree);
    }

    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {}

    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {}

    fn visit_unit_field(&mut self, ident_name: &String) {}
}
