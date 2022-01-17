use syn::{Attribute, FieldsNamed, FieldsUnnamed, File, Item, ItemEnum, ItemUse, Variant};

pub trait Visitor {
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
    fn visit_attribute(&mut self, attribute: &Attribute) {}
    fn visit_item_enum(&mut self, item_enum: &ItemEnum);
    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant);
    fn visit_item_use(&mut self, item_use: &ItemUse);
    fn visit_named_field(&mut self, ident_name: &String, field_named: &FieldsNamed);
    fn visit_unnamed_field(&mut self, ident_name: &String, field_unnamed: &FieldsUnnamed);
    fn visit_unit_field(&mut self, ident_name: &String);
}
