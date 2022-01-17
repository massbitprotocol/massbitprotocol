use crate::config::IndexerConfig;
use crate::parser::visitor::Visitor;
use syn::__private::ToTokens;
use syn::{
    FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemExternCrate, ItemMod, ItemUse, UseTree,
    Variant,
};

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
        let mut mods = Vec::default();
        self.parse_use_tree(&item_use.tree);
    }
    fn visit_item_extern_crate(&mut self, item_extern_create: &ItemExternCrate) {}
    fn visit_item_module(&mut self, item_module: &ItemMod) {}
    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {}

    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {}

    fn visit_unit_field(&mut self, ident_name: &String) {}
}
impl Definitions {
    fn parse_use_tree(&mut self, mods: &mut Vec<String>, use_tree: &UseTree) {
        match use_tree {
            UseTree::Path(path) => {
                println!("{:?}", &path);
            }
            UseTree::Name(name) => {}
            UseTree::Rename(_) => {}
            UseTree::Glob(_) => {}
            UseTree::Group(group) => {}
        }
    }
}
