use syn::{Attribute, File, Item, ItemEnum, ItemUse, Variant};

pub trait Visitor<T> {
    fn visit_file(&mut self, file: &File) -> T;
    fn visit_item(&mut self, item: &Item) -> T;
    fn visit_attribute(&mut self, attribute: &Attribute) -> T;
    fn visit_item_enum(&mut self, item_enum: &ItemEnum) -> T;
    fn visit_item_variant(&mut self, item_enum: &ItemEnum, variant: &Variant) -> T;
    fn visit_item_use(&mut self, item_use: &ItemUse) -> T;
}
