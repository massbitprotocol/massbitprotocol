use crate::generator::Generator;
use crate::schema::{Schema, Variant};
use lazy_static::lazy_static;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

lazy_static! {
    // https://www.codingame.com/playgrounds/365/getting-started-with-rust/primitive-data-types
    // pub static ref PRIMITIVE_DATA_TYPES: Vec<&'static str> = vec![
    //     "bool", "char", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "isize", "usize", "f32",
    //     "f64", "str", //Not support yet
    //     "NonZeroU8", "NonZeroU32", "NonZeroU64", "NonZeroU128",
    //     "NonZeroI8", "NonZeroI32", "NonZeroI64", "NonZeroI128",
    //     "array", "slice", "tuple",
    // ];

    // https://kotiri.com/2018/01/31/postgresql-diesel-rust-types.html
    pub static ref MAPPING_RUST_TYPES_TO_DB: HashMap<&'static str, &'static str> = HashMap::from([
        ("bool", "Bool"),
        //The graph generator postgres sql only handles with bigint
        ("i8", "BigInt"),
        ("u8", "BigInt"),
        ("i16", "BigInt"),
        ("u16", "BigInt"),
        ("NonZeroU8", "BigInt"),
        ("NonZeroU16", "BigInt"),
        ("NonZeroI8", "BigInt"),
        ("NonZeroI16", "BigInt"),

        ("i32", "Integer"),
        ("u32", "Integer"),
        ("NonZeroI32", "Integer"),
        ("NonZeroU32", "Integer"),


        ("i64", "BigInt"),
        ("u64", "BigInt"),
        ("isize", "BigInt"),
        ("usize", "BigInt"),
        ("usize", "BigInt"),
        ("NonZeroU64", "BigInt"),
        ("NonZeroU128", "BigInt"),
        ("NonZeroUsize", "BigInt"),
        ("NonZeroI64", "BigInt"),
        ("NonZeroI128", "BigInt"),
        ("NonZeroIsize", "BigInt"),

        ("f32", "Float"),

        ("f64", "Double"),

        ("str", "String"),
        ("String", "String"),
        ("char", "String"),
    ]);
    pub static ref MAPPING_DB_TYPES_TO_RUST: HashMap<&'static str, &'static str> = HashMap::from([
        ("Bool", "bool"),

        ("SmallInt", "i64"),
        ("Integer", "i64"),
        ("BigInt", "i64"),

        ("Float", "f32"),

        ("Double", "f64"),

        ("String", "String"),
    ]);

    pub static ref DEFAULT_TYPE_DB : &'static str = "String";
}

impl<'a> Generator<'a> {
    pub fn generate_graphql_schema(&self, schema: &Schema) -> String {
        let mut out = String::new();
        if let Some(variants) = &schema.variants {
            // List instruction
            for variant in variants {
                // Write table if there is inner_type
                let variant_entity = self.generate_variant_entity(variant, &self.definitions);
                writeln!(out, "{}", &variant_entity);
                // Get definitions
                // if let Some(sub_schema) = self.definitions.get(&inner_type) {
                //     // get a table corresponding to sub_schema
                //     let str_entity: String =
                //         Schema::gen_entity_db(sub_schema, inner_type, instruction.name);
                //     writeln!(out, "{}", str_entity);
                // } else if MAPPING_RUST_TYPES_TO_DB.contains_key(inner_type.as_str()) {
                //     let str_entity: String =
                //         Schema::gen_entity_db(&Schema::default(), inner_type, instruction.name);
                //     writeln!(out, "{}", str_entity);
                // }
            }
        }
        out
    }
    fn generate_variant_entity(
        &self,
        variant: &Variant,
        definitions: &BTreeMap<String, Schema>,
    ) -> String {
        let mut entity_properties: Vec<String> = vec![String::from("id: ID!")];
        //Account assigment
        if let Some(accounts) = &variant.accounts {
            for account in accounts {
                entity_properties.push(format!("\t{}: String", account.name));
            }
        }
        if let Some(inner_type) = &variant.inner_type {
            if let Some(def) = definitions.get(inner_type.as_str()) {
                if let Some(properties) = &def.properties {
                    for property in properties {
                        let db_type = MAPPING_RUST_TYPES_TO_DB
                            .get(property.data_type.as_str())
                            .unwrap_or(&*DEFAULT_TYPE_DB);
                        if property.array_length.is_some() {
                            entity_properties.push(format!("\t{}: [{}]", &property.name, db_type));
                        } else {
                            entity_properties.push(format!("\t{}: {}", &property.name, db_type));
                        }
                    }
                }
            } else if let Some(db_type) = MAPPING_RUST_TYPES_TO_DB.get(inner_type.as_str()) {
                //Inner type is primitive. Store is as an value field
                entity_properties.push(format!("\tvalue: {}", db_type));
            }
        }
        format!(
            r#"type {} @entity {{
    {entity_properties}
}}"#,
            &variant.name,
            entity_properties = entity_properties.join(",\n")
        )
    }
    // pub fn gen_entity_db(schema: &Schema, entity_type: String, entity_name: String) -> String {
    //     let mut entity_properties: Vec<String> = vec![String::from("\tid: ID!")];
    //     match MAPPING_RUST_TYPES_TO_DB.get(entity_type.as_str()) {
    //         Some(db_type) => {
    //             entity_properties.push(format!("\tvalue: {}", db_type));
    //         }
    //         None => {
    //             if let Some(properties) = &schema.properties {
    //                 for property in properties {
    //                     match property.array_length {
    //                         Some(_array_length) => {
    //                             let db_type = MAPPING_RUST_TYPES_TO_DB
    //                                 .get(property.data_type.as_str())
    //                                 .unwrap_or(&*DEFAULT_TYPE_DB);
    //                             entity_properties
    //                                 .push(format!("\t{}: [{}]", &property.name, db_type));
    //                         }
    //                         None => {
    //                             let db_type = MAPPING_RUST_TYPES_TO_DB
    //                                 .get(property.data_type.as_str())
    //                                 .unwrap_or(&*DEFAULT_TYPE_DB);
    //                             entity_properties
    //                                 .push(format!("\t{}: {}", &property.name, db_type));
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     };
    //     format!(
    //         r#"type {} @entity {{
    //             {entity_properties}
    //         }}"#,
    //         &entity_name,
    //         entity_properties = entity_properties.join(",\n")
    //     )
    // }
}
