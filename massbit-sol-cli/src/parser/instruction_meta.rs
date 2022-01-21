use super::visitor::Visitor;
use crate::config::IndexerConfig;
use crate::schema::AccountInfo;
use std::collections::HashMap;
use syn::{FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Variant};

#[derive(Default)]
pub struct InstructionMeta {
    pub config_path: String,
    pub config: IndexerConfig,
    variants: HashMap<String, Vec<AccountInfo>>,
}

impl InstructionMeta {
    pub fn new(config_path: String, config: IndexerConfig) -> Self {
        Self {
            config_path,
            config,
            variants: HashMap::default(),
        }
    }
}

impl Visitor for InstructionMeta {
    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) {
        // let variant = format!(
        //     r#""{variant_name}":[{{"index":0,"name":"account_name"}}]"#,
        //     variant_name = &variant.ident.to_string()
        // );
        //self.variants.push(variant);
        self.variants.insert(
            variant.ident.to_string(),
            vec![AccountInfo {
                index: 0,
                name: "account_name".to_string(),
            }],
        );
    }
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        let ident = item_enum.ident.to_string();
        if self.config.main_instruction.as_str() == ident.as_str() {
            item_enum
                .variants
                .iter()
                .for_each(|variant| self.visit_item_variant(item_enum, variant));
        }
    }

    fn visit_item_use(&mut self, item_use: &ItemUse) {}

    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {}

    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {}

    fn visit_unit_field(&mut self, ident_name: &String) {}

    fn create_content(&self) -> String {
        //         format!(
        //             r#"{{
        //     {variants}
        // }}"#,
        //             variants = self.variants.join(",\n\t")
        //         )
        serde_json::to_string_pretty(&self.variants).unwrap_or_default()
    }

    fn create_dir_path(&self) -> String {
        format!("{}", &self.config_path)
    }

    fn build(&self) {}
}
