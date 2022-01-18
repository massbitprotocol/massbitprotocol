use syn::ItemStruct;

#[derive(Debug, Clone)]
pub struct ItemStructDef {
    pub mods: Vec<String>,
    pub item_struct: ItemStruct,
}

impl ItemStructDef {
    pub fn new(mods: Vec<String>, item_struct: ItemStruct) -> Self {
        Self { mods, item_struct }
    }
}
