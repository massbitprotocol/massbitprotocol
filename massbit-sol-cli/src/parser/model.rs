use crate::consts::{MAPPING_RUST_TYPES_TO_DB, PRIMITIVE_DATA_TYPES};
use crate::parser::Definitions;
use std::fmt::Write;
use std::ops::Add;
use syn::__private::ToTokens;
use syn::{Fields, ItemEnum, ItemStruct, PathArguments, PathSegment, Type, Visibility};

#[derive(Debug, Clone)]
pub enum ItemType {
    ItemStruct(ItemStruct),
    ItemEnum(ItemEnum),
}
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub package_name: String,
    pub mods: Vec<String>,
    pub item_type: ItemType,
}

impl ItemType {
    pub fn create_unpack_function(&self, definitions: &Definitions) -> Vec<String> {
        match self {
            ItemType::ItemStruct(item_struct) => {
                self.create_struct_unpack(item_struct, definitions)
            }
            ItemType::ItemEnum(item_enum) => self.create_enum_unpack(item_enum, definitions),
        }
    }
    pub fn create_struct_unpack(
        &self,
        item_struct: &ItemStruct,
        definitions: &Definitions,
    ) -> Vec<String> {
        let mut assigments = Vec::<String>::new();
        match &item_struct.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .filter(|field| matches!(field.vis, Visibility::Public(_)))
                .for_each(|field| match &field.ty {
                    Type::Array(arr) => {}
                    Type::BareFn(_) => {}
                    Type::Group(_) => {}
                    Type::ImplTrait(_) => {}
                    Type::Infer(_) => {}
                    Type::Macro(_) => {}
                    Type::Never(_) => {}
                    Type::Paren(_) => {}
                    Type::Path(type_path) => {
                        //handle single segments
                        let field = field.ident.to_token_stream().to_string();
                        let value = match type_path.path.segments.first() {
                            None => String::from("Value::Null"),
                            Some(segment) => {
                                let ident = segment.ident.to_string();
                                let str = ident.as_str();

                                match ident.as_str() {
                                    id @ "Vec" | id @ "Option" => match &segment.arguments {
                                        PathArguments::None => String::from("Value::Null"),
                                        PathArguments::AngleBracketed(arg) => {
                                            let type_name = arg.args.to_token_stream().to_string();
                                            if PRIMITIVE_DATA_TYPES.contains(&type_name.as_str())
                                            {
                                                format!("Value::from(input.{field})", field = field)
                                            } else if let Some(type_def) = definitions.get_item_def(&type_name) {
                                                //spread fields
                                                println!("{:?}", type_def);
                                                if id == "Vec" {
                                                    format!(r#"Value::from(input.{field}.iter().map(|item|{{
                                                        format!("{{:?}}",item)
                                                       }}).collect::<Vec<String>>())"#, field = field)
                                                } else {
                                                    format!(r#"Value::from(input.{field}.map(|item|{{
                                                        format!("{{:?}}",item)
                                                       }}))"#, field = field)
                                                }

                                            } else {
                                                String::from("Value::Null")
                                            }

                                        }
                                        PathArguments::Parenthesized(_) => String::from("Value::Null")
                                    },
                                    id => {
                                        if PRIMITIVE_DATA_TYPES.contains(&str)
                                            {
                                            format!("Value::from(input.{field})", field = field)
                                        } else if let Some(type_def) = definitions.get_item_def(&ident) {
                                            type_def.create_unpack_value(&field, definitions)
                                        } else {
                                            format!(
                                                r#"Value::String(format!("{{:?}}", input.{field}))"#,
                                                field = field
                                            )
                                        }
                                    }
                                }

                            }
                        };

                        assigments.push(format!(
                            r#"transport_value.set_value("{field}", {value});"#,
                            field = field,
                            value = value
                        ));
                    }
                    Type::Ptr(_) => {}
                    Type::Reference(_) => {}
                    Type::Slice(_) => {}
                    Type::TraitObject(_) => {}
                    Type::Tuple(_) => {}
                    Type::Verbatim(_) => {}
                    Type::__TestExhaustive(_) => {}
                }),
            Fields::Unnamed(fields) => fields.unnamed.iter().for_each(|field| {
                // println!(
                //     "{:?}::{:?}",
                //     field.ident.to_token_stream().to_string(),
                //     &field.ty
                // );
            }),
            Fields::Unit => {}
        }
        assigments
    }
    pub fn create_enum_unpack(
        &self,
        item_enum: &ItemEnum,
        definitions: &Definitions,
    ) -> Vec<String> {
        let mut assigments = Vec::<String>::new();
        item_enum.variants.iter().for_each(|variant| {
            let field = variant.ident.to_token_stream().to_string();
            assigments.push(format!(
                r#"transport_value.set_value("{field}", {value});"#,
                field = field,
                value = "Value::Null"
            ));
        });
        assigments
    }
}
impl ItemDef {
    pub fn new(package_name: String, mods: Vec<String>, item_type: ItemType) -> Self {
        Self {
            package_name,
            mods,
            item_type,
        }
    }
    pub fn get_module_path(&self) -> String {
        let mut path = self.package_name.clone().replace("-", "_");
        for module in self.mods.iter() {
            let str = module.as_str();
            if str != "crate" {
                let _ = write!(path, "::{}", str);
            }
        }
        path
    }
    pub fn create_unpack_function(
        &self,
        entity_name: &String,
        definitions: &Definitions,
    ) -> String {
        let assigments = self.item_type.create_unpack_function(definitions);
        format!(
            r#"
            let mut transport_value = TransportValue::new("{entity_name}");
            {assigments}
            Ok(transport_value)"#,
            assigments = assigments.join(";"),
            entity_name = entity_name
        )
    }
    pub fn create_unpack_value(&self, field: &String, definitions: &Definitions) -> String {
        match &self.item_type {
            ItemType::ItemStruct(item_truct) => r#"Value::Null"#.to_string(),
            ItemType::ItemEnum(item_enum) => {
                let enum_ident = item_enum.ident.to_string();
                let mut variants = Vec::<String>::new();
                item_enum.variants.iter().for_each(|variant| {
                    variants.push(format!(
                        r#"{module}::{enum_ident}::{variant_name} => {{ Value::from("{variant_name}")}}"#,
                        module = self.get_module_path(),
                        enum_ident = enum_ident,
                        variant_name = variant.ident.to_string()
                    ))
                });
                format!(
                    r#"match input.{field} {{
                    {variants}
                }}"#,
                    field = field,
                    variants = variants.join("")
                )
            }
        }
    }
    pub fn create_entity_fields(&self, definitions: &Definitions) -> Vec<String> {
        if let ItemType::ItemStruct(item_struct) = &self.item_type {
            let mut fields = Vec::new();
            for field in item_struct.fields.iter() {
                let field_type = self.create_entity_field_type(definitions, &field.ty);
                if field_type.len() > 0 && field.ident.is_some() {
                    fields.push(format!(
                        "{}: {}",
                        field.ident.as_ref().unwrap(),
                        &field_type
                    ))
                }
            }
            fields
        } else {
            Vec::default()
        }
    }
    pub fn create_entity_field_type(&self, definitions: &Definitions, field_type: &Type) -> String {
        let mut out = String::new();
        match field_type {
            Type::Array(ta) => {}
            Type::BareFn(_) => {}
            Type::Group(_) => {}
            Type::ImplTrait(_) => {}
            Type::Infer(_) => {}
            Type::Macro(_) => {}
            Type::Never(_) => {}
            Type::Paren(_) => {}
            Type::Path(tp) => {
                if let Some(segment) = tp.path.segments.first() {
                    let ident = segment.ident.to_string();
                    match ident.as_str() {
                        id @ "Vec" | id @ "Option" => match &segment.arguments {
                            PathArguments::None => {}
                            PathArguments::AngleBracketed(arg) => {
                                let type_name = arg.args.to_token_stream().to_string();
                                if let Some(graphql_type) =
                                    MAPPING_RUST_TYPES_TO_DB.get(type_name.as_str())
                                {
                                    if id == "Vec" {
                                        let _ = write!(&mut out, "[{}]", graphql_type);
                                    } else {
                                        let _ = write!(&mut out, "{}", graphql_type);
                                    }
                                }
                            }
                            PathArguments::Parenthesized(_) => {}
                        },
                        id => {
                            if let Some(graphql_type) = MAPPING_RUST_TYPES_TO_DB.get(&id) {
                                let _ = write!(&mut out, "{}", graphql_type);
                            } else if let Some(item_def) = definitions.get_item_def(&ident) {
                                // println!(
                                //     "{:?}",
                                //     item_def.item_type.create_unpack_function(definitions)
                                // );
                                out = String::from("String");
                            }
                        }
                    }
                }
            }
            Type::Ptr(_) => {}
            Type::Reference(_) => {}
            Type::Slice(_) => {}
            Type::TraitObject(_) => {}
            Type::Tuple(_) => {}
            Type::Verbatim(_) => {}
            Type::__TestExhaustive(_) => {}
        };
        out
    }
}
