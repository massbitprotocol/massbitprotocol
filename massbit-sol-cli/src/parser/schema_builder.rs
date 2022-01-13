use crate::parser::visittor::Visitor;
use crate::schema;
use crate::schema::{Schema, VariantArray};
use std::path::Path;
use std::{fs, io};
use syn::__private::ToTokens;
use syn::{Attribute, Field, Fields, File, Item, ItemEnum, ItemUse, Type, Variant};

pub struct SchemaBuilder<'a> {
    pub instruction_path: &'a str,
    pub output_dir: &'a str,
    name: &'a str,
    enums: Vec<String>,
    schema: Schema,
    context: VisitorContext<'a>,
}
impl<'a> SchemaBuilder<'a> {
    fn default() -> Self {
        Self {
            instruction_path: "",
            output_dir: "",
            name: "",
            enums: Vec::default(),
            schema: Default::default(),
            context: VisitorContext::default(),
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
    pub fn with_enums(mut self, enums: Option<Vec<String>>) -> Self {
        if let Some(values) = enums {
            values.iter().for_each(|val| {
                self.enums.push(val.clone());
            })
        }
        self
    }
    pub fn with_name(mut self, name: &'a str) -> Self {
        self.name = name;
        self.schema.name = Some(String::from(self.name));
        self
    }
    pub fn build(&mut self) {
        let input_content = fs::read_to_string(self.instruction_path).unwrap_or_else(|_| {
            panic!(
                "Something went wrong reading the file {}",
                &self.instruction_path
            )
        });
        if let Ok(ast) = syn::parse_file(&input_content) {
            self.visit_file(&ast);
            if let Ok(content) = serde_json::to_string_pretty(&self.schema) {
                let output_path = format!("{}/{}", self.output_dir, self.name);
                if fs::create_dir_all(&output_path).is_ok() {
                    let output_path = format!("{}/{}/instruction.json", self.output_dir, self.name);
                    let path = Path::new(output_path.as_str());
                    self.write_to_file(path, &content);
                }
            }
        };
    }
    pub fn write_to_file<P: ?Sized + AsRef<Path>>(
        &self,
        output_path: &P,
        content: &str,
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
#[derive(Default)]
pub struct VisitorContext<'a> {
    current_item: Option<&'a Item>,
}
impl<'a> SchemaBuilder<'a> {
    fn create_variant(&self, item_variant: &Variant) -> schema::Variant {
        let mut variant = schema::Variant {
            name: item_variant.ident.to_string(),
            value: None,
            inner_name: None,
            inner_type: None,
            inner_scope: None,
            description: None,
            offset: None,
            variant_tag: 0,
            accounts: None,
        };
        for field in item_variant.fields.iter() {
            match &field.ty {
                Type::Array(_) => {}
                Type::BareFn(_) => {}
                Type::Group(_) => {}
                Type::ImplTrait(_) => {}
                Type::Infer(_) => {}
                Type::Macro(_) => {}
                Type::Never(_) => {}
                Type::Paren(_) => {}
                Type::Path(path) => {
                    variant.inner_type = path
                        .path
                        .segments
                        .first()
                        .and_then(|seg| Some(seg.ident.to_string()));
                }
                Type::Ptr(_) => {}
                Type::Reference(_) => {}
                Type::Slice(_) => {}
                Type::TraitObject(_) => {}
                Type::Tuple(_) => {}
                Type::Verbatim(_) => {}
                Type::__TestExhaustive(_) => {}
            }
        }

        variant
    }
    fn create_definition(&self, item_variant: &Variant) -> Option<schema::Schema> {
        None
    }
}
#[doc = " Instructions supported by the Fraction program."]
impl<'a> Visitor<()> for SchemaBuilder<'a> {
    #[doc = " Instructions supported by the Fraction program."]
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

    fn visit_attribute(&mut self, attribute: &Attribute) {
        todo!()
    }

    fn visit_item_enum(&mut self, item_enum: &ItemEnum) {
        println!("{:?}", item_enum.to_token_stream().to_string());
        let ident = item_enum.ident.to_string();
        if self.enums.contains(&ident) && self.enums.len() == 1 {
            self.schema.name = Some(ident.clone());
            self.schema.variants = Some(Vec::new());
        }
        for attr in item_enum.attrs.iter() {
            println!("{:?}", attr);
            println!("{:?}", attr.to_token_stream().to_string());
        }
        println!(
            "Enum name {:?}, Variant number: {}",
            item_enum.ident,
            item_enum.variants.len()
        );
        item_enum
            .variants
            .iter()
            .for_each(|variant| self.visit_item_variant(item_enum, variant));
    }
    fn visit_item_variant(&mut self, item: &ItemEnum, item_variant: &Variant) {
        let item_ident = item.ident.to_string();
        if self.enums.contains(&item_ident) {
            let variant = self.create_variant(item_variant);
            let definition = self.create_definition(item_variant);
            if let Some(def) = definition {
                self.schema
                    .definitions
                    .insert(item_variant.ident.to_string(), def);
            }

            self.schema.variants.as_mut().unwrap().push(variant);
        }

        // println!("Variant attrs {:?}", &item_variant.attrs);
        // println!("Variant ident {:?}", &item_variant.ident);
        // println!("Variant fields {:?}", &item_variant.fields);
        // println!("Variant discriminant {:?}", &item_variant.discriminant);
    }
    fn visit_item_use(&mut self, item_use: &ItemUse) {}
}
impl<'a> SchemaBuilder<'a> {}
