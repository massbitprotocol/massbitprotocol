use crate::config::IndexerConfig;
use crate::parser::model::{ItemDef, ItemType};
use crate::parser::visitor::Visitor;
use std::collections::HashMap;
use std::fs;
use syn::__private::ToTokens;
use syn::{
    FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemExternCrate, ItemMod, ItemStruct,
    ItemUse, UseTree, Variant,
};

#[derive(Default)]
pub struct Definitions {
    pub config: IndexerConfig,
    parsed_modules: Vec<String>,
    defined_items: HashMap<String, ItemDef>,
    pub current_paths: Vec<String>,
}

impl Definitions {
    pub fn new(config: IndexerConfig) -> Self {
        Self {
            config,
            parsed_modules: vec![],
            defined_items: Default::default(),
            current_paths: vec![],
        }
    }
    pub fn get_item_def(&self, item_def: &String) -> Option<&ItemDef> {
        self.defined_items.get(item_def)
    }
}
impl Visitor for Definitions {
    fn visited_module(&mut self, path: &String) -> bool {
        self.parsed_modules.contains(path)
    }
    fn mark_module_visited(&mut self, path: &String) {
        self.parsed_modules.push(path.clone());
    }
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        self.defined_items.insert(
            item_enum.ident.to_string(),
            ItemDef::new(
                self.config.package_name.clone(),
                self.current_paths.clone(),
                ItemType::ItemEnum(item_enum.clone()),
            ),
        );
    }

    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) {}

    fn visit_item_use(&mut self, item_use: &ItemUse) {
        let last_mods = self.current_paths.clone();
        self.current_paths = Vec::default();
        self.parse_use_tree(&item_use.tree);
        self.current_paths = last_mods;
    }
    fn visit_item_extern_crate(&mut self, item_extern_create: &ItemExternCrate) {}
    fn visit_item_module(&mut self, item_module: &ItemMod) {}
    fn visit_item_struct(&mut self, item_struct: &ItemStruct) {
        self.defined_items.insert(
            item_struct.ident.to_string(),
            ItemDef::new(
                self.config.package_name.clone(),
                self.current_paths.clone(),
                ItemType::ItemStruct(item_struct.clone()),
            ),
        );
    }
    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed) {}

    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed) {}

    fn visit_unit_field(&mut self, ident_name: &String) {}

    fn create_content(&self) -> String {
        String::new()
    }

    fn create_dir_path(&self) -> String {
        String::new()
    }

    fn build(&self) {}
}
impl Definitions {
    fn parse_use_tree(&mut self, use_tree: &UseTree) {
        match use_tree {
            UseTree::Path(path) => {
                let current_paths = self.current_paths.clone();
                let ident = path.ident.to_string();
                self.current_paths.push(ident);
                self.parse_use_tree(path.tree.as_ref());
                self.current_paths = current_paths;
            }
            UseTree::Name(name) => {
                if self.current_paths.len() > 0
                    && self.current_paths.get(0).unwrap().as_str() == "crate"
                {
                    let mut mods = self.current_paths.clone();
                    mods.remove(0);
                    let base_dir = self.config.smart_contract_source.clone();
                    self.visit_module(&base_dir, &mods);
                }
            }
            UseTree::Rename(_) => {}
            UseTree::Glob(_) => {}
            UseTree::Group(group) => {
                for tree in group.items.iter() {
                    self.parse_use_tree(tree);
                }
            }
        }
    }
    fn parse_mod_file(&mut self, path: String) {
        let content = fs::read_to_string(path.as_str())
            .unwrap_or_else(|_| panic!("Something went wrong reading the file {}", &path));
        println!("{:?}", &content);
        if let Ok(ast) = syn::parse_file(&content) {}
    }
}
