use crate::schema::{PropertyArray, Schema, VariantArray};
use lazy_static::lazy_static;
use std::collections::HashMap;
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

        ("i8", "SmallInt"),
        ("u8", "SmallInt"),
        ("i16", "SmallInt"),
        ("u16", "SmallInt"),
        ("NonZeroU8", "SmallInt"),
        ("NonZeroU16", "SmallInt"),
        ("NonZeroI8", "SmallInt"),
        ("NonZeroI16", "SmallInt"),

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

impl Schema {
    pub fn gen_graphql_schema(&self) -> String {
        let mut out = String::new();
        // println!("Schema: {:#?}", self);
        if let Some(instructions) = self.variants.clone() {
            // List instruction
            for instruction in instructions {
                // Write table if there is inner_type
                if let Some(inner_type) = instruction.inner_type {
                    // if let Ok() = self.definitions.get(inner_type){
                    //
                    // }
                    println!("{}({})", &instruction.name, &inner_type);
                    // Get definitions
                    if let Some(sub_schema) = self.definitions.get(&inner_type) {
                        // get a table corresponding to sub_schema
                        let str_entity: String =
                            Schema::gen_entity_db(sub_schema, inner_type, instruction.name);
                        writeln!(out, "{}", str_entity);
                    } else if MAPPING_RUST_TYPES_TO_DB.contains_key(inner_type.as_str()) {
                        let str_entity: String =
                            Schema::gen_entity_db(&Schema::default(), inner_type, instruction.name);
                        writeln!(out, "{}", str_entity);
                    }
                }
            }
        }
        out
    }

    pub fn gen_entity_db(schema: &Schema, entity_type: String, entity_name: String) -> String {
        let mut out = String::new();
        // Write entity name
        write!(
            out,
            r#"type {} @entity {{
    id: ID!"#,
            &entity_name
        );
        // if it is primitive type
        match MAPPING_RUST_TYPES_TO_DB.get(entity_type.as_str()) {
            Some(db_type) => {
                write!(
                    out,
                    r#"
    value: {}"#,
                    db_type
                );
            }
            None => {
                if let Some(properties) = &schema.properties {
                    for property in properties {
                        let db_type = MAPPING_RUST_TYPES_TO_DB
                            .get(property.data_type.as_str())
                            .unwrap_or(&*DEFAULT_TYPE_DB);
                        write!(
                            out,
                            r#"
    {}: {}"#,
                            property.name, db_type
                        );
                    }
                }
            }
        };
        write!(
            out,
            r#"
 }}
 "#
        );

        out
    }
}
