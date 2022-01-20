use std::fs;
use syn::{
    Attribute, Fields, FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemExternCrate, ItemMod,
    ItemStruct, ItemUse, Variant,
};

pub trait Visitor {
    fn visit_module(&mut self, base_dir: &str, paths: &Vec<String>) {
        let module_path = format!("{}/src/{}.rs", base_dir, paths.join(","));
        if !self.visited_module(&module_path) {
            let content = fs::read_to_string(module_path.as_str()).unwrap_or_else(|_| {
                panic!("Something went wrong reading the file {}", &module_path)
            });
            self.mark_module_visited(&module_path);
            if let Ok(ast) = syn::parse_file(&content) {
                self.visit_file(&ast);
            }
        }
    }
    fn visited_module(&mut self, path: &String) -> bool {
        false
    }
    fn mark_module_visited(&mut self, path: &String) {}
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
            Item::ExternCrate(ex_crate) => {
                self.visit_item_extern_crate(ex_crate);
            }
            Item::Fn(_) => {}
            Item::ForeignMod(_) => {}
            Item::Impl(_) => {}
            Item::Macro(_) => {}
            Item::Macro2(_) => {}
            Item::Mod(module) => {
                self.visit_item_module(module);
            }
            Item::Static(_) => {}
            Item::Struct(item_struct) => {
                self.visit_item_struct(item_struct);
            }
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
    fn visit_attribute(&mut self, attribute: &Attribute) {}
    fn visit_item_enum(&mut self, item_enum: &ItemEnum);
    fn visit_item_extern_crate(&mut self, item_extern_create: &ItemExternCrate) {}
    fn visit_item_module(&mut self, item_module: &ItemMod) {}
    fn visit_item_struct(&mut self, item_struct: &ItemStruct) {}
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
    }
    fn visit_item_use(&mut self, item_use: &ItemUse);
    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed);
    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed);
    fn visit_unit_field(&mut self, ident_name: &String);
    fn create_content(&self) -> String;
    fn create_dir_path(&self) -> String;
    fn write_output(&self, file_name: &str) {
        let dir_path = self.create_dir_path();
        if std::fs::create_dir_all(&dir_path).is_ok() {
            let output_path = format!("{}/{}", &dir_path, file_name);
            let content = self.create_content();
            match std::fs::write(&output_path, &content) {
                Ok(_) => {
                    use std::process::Command;
                    let _ = Command::new("rustfmt").arg(output_path).output();
                }
                Err(err) => {
                    log::error!("{:?}", &err);
                }
            }
        }
    }
}
