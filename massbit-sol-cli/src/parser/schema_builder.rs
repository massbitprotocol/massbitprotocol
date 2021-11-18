use crate::schema::Schema;
use std::path::Path;
use std::{fs, io};
use syn::{Attribute, DeriveInput, Item, ItemEnum, ItemUse, Variant};

pub struct SchemaBuilder<'a> {
    pub instruction_path: &'a str,
    pub output_dir: &'a str,
}
impl<'a> SchemaBuilder<'a> {
    fn default() -> Self {
        Self {
            instruction_path: "",
            output_dir: "",
        }
    }
    pub fn builder() -> SchemaBuilder<'a> {
        SchemaBuilder::default()
    }
    pub fn with_instruction_path(mut self, path: &'a str) -> Self {
        self.instruction_path = path;
        self
    }
    pub fn with_output_dir(mut self, output_dir: &'a str) -> Self {
        self.output_dir = output_dir;
        self
    }
    pub fn build(&self) {
        let input_content = fs::read_to_string(self.instruction_path).expect(
            format!(
                "Something went wrong reading the file {}",
                &self.instruction_path
            )
            .as_str(),
        );
        // let mut file = File::open(INPUT_RUST_CODE)?;
        // let mut content = String::new();
        // file.read_to_string(&mut content)?;

        match syn::parse_file(&input_content) {
            Ok(ast) => {
                let schema = Schema::default();
                if let Some(shebang) = ast.shebang {
                    println!("{}", shebang);
                }
                ast.items.iter().for_each(|item| self.process_item(item));
                ast.attrs.iter().for_each(|item| println!("{:?}", item));
            }
            Err(_) => {}
        };
    }
    fn process_item(&self, item: &Item) {
        match item {
            Item::Const(_) => {}
            Item::Enum(item_enum) => {
                self.process_item_enum(item_enum);
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
                self.process_item_use(item_use);
            }
            Item::Verbatim(_) => {}
            Item::__TestExhaustive(_) => {}
        }
        //println!("{:?}", item)
    }
    fn process_item_enum(&self, item_enum: &ItemEnum) {
        println!("Variant number: {}", item_enum.variants.len());
        item_enum
            .variants
            .iter()
            .for_each(|variant| self.process_item_variant(variant));
    }
    fn process_item_variant(&self, variant: &Variant) {
        println!("Variant attrs {:?}", &variant.attrs);
        println!("Variant ident {:?}", &variant.ident);
        println!("Variant fields {:?}", &variant.fields);
        println!("Variant discriminant {:?}", &variant.discriminant);
    }
    fn process_attribute(&self, attribute: &Attribute) {
        //println!("{:?}", attribute)
    }
    fn process_item_use(&self, item_use: &ItemUse) {}
    pub fn write_to_file<P: ?Sized + AsRef<Path>>(
        &self,
        output_path: &P,
        content: &String,
    ) -> io::Result<()> {
        match fs::write(output_path, content) {
            Ok(_) => {
                log::info!(
                    "Write content to file {:?} successfully",
                    &output_path.as_ref().as_os_str()
                );
                Ok(())
            }
            e @ Err(_) => {
                log::info!(
                    "Write content to file {:?} fail. {:?}",
                    &output_path.as_ref().as_os_str(),
                    &e
                );
                e
            }
        }
    }
}
