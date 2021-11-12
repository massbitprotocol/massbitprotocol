use proc_macro2::{Ident, Span, TokenStream};
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use syn::{Attribute, DeriveInput, Item, ItemEnum, Variant};

const INPUT_RUST_CODE: &str =
    "/home/viettai/Massbit/solana-smart-contracts/mango-v3/program/src/instruction.rs";

fn main() {
    let input_content =
        fs::read_to_string(INPUT_RUST_CODE).expect("Something went wrong reading the file");
    // let mut file = File::open(INPUT_RUST_CODE)?;
    // let mut content = String::new();
    // file.read_to_string(&mut content)?;

    if let Ok(ast) = syn::parse_file(&input_content) {
        if let Some(shebang) = ast.shebang {
            println!("{}", shebang);
        }
        ast.items.iter().for_each(|item| process_item(item));
        ast.attrs.iter().for_each(|item| println!("{:?}", item));
    }
    // if let Ok(input) = TokenStream::from_str(input_content.as_str()) {
    //     let derive_input = syn::parse2(input);
    //     println!("{:?}", derive_input);
    // }
}
fn process_item(item: &Item) {
    match item {
        Item::Const(_) => {}
        Item::Enum(item_enum) => {
            process_item_enum(item_enum);
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
        Item::Use(_) => {}
        Item::Verbatim(_) => {}
        Item::__TestExhaustive(_) => {}
    }
    //println!("{:?}", item)
}
fn process_item_enum(item_enum: &ItemEnum) {
    println!("Variant number: {}", item_enum.variants.len());
    item_enum
        .variants
        .iter()
        .for_each(|variant| process_item_variant(variant));
}
fn process_item_variant(variant: &Variant) {
    println!("{:?}", variant);
}
fn process_attribute(attribute: &Attribute) {
    //println!("{:?}", attribute)
}
