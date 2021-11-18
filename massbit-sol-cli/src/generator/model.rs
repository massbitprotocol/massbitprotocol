use crate::generator::graphql::{
    DEFAULT_TYPE_DB, MAPPING_DB_TYPES_TO_RUST, MAPPING_RUST_TYPES_TO_DB,
};
use crate::schema::Schema;
use std::fmt::Write;

impl Schema {
    pub fn gen_models(&self) -> String {
        let mut out = String::new();
        // println!("Schema: {:#?}", self);
        writeln!(out, "{}", Schema::gen_model_file_header());

        if let Some(instructions) = self.variants.clone() {
            // List instruction
            for instruction in instructions {
                // Write table if there is inner_type
                if let Some(inner_type) = instruction.inner_type {
                    // Get definitions
                    if let Some(sub_schema) = self.definitions.get(&inner_type) {
                        // get a table corresponding to sub_schema
                        let str_entity: String =
                            Schema::gen_entity_struct(sub_schema, &inner_type, &instruction.name);
                        writeln!(out, "{}", str_entity);
                    } else if MAPPING_RUST_TYPES_TO_DB.contains_key(inner_type.as_str()) {
                        let str_entity: String = Schema::gen_entity_struct(
                            &Schema::default(),
                            &inner_type,
                            &instruction.name.clone(),
                        );
                        writeln!(out, "{}", str_entity);
                    }
                    writeln!(
                        out,
                        "{}",
                        Schema::gen_model_file_function(&instruction.name)
                    );
                }
            }
        }

        out
    }

    pub fn gen_model_file_header() -> String {
        let mut out = String::new();
        // Write entity name
        write!(
            out,
            r#"
            use crate::STORE;
            use crate::{{Entity, EntityFilter, EntityOrder, EntityRange, Value}};
            use crate::{{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom}};
            pub use massbit_drive::{{FromEntity, ToMap}};
            use std::collections::HashMap;
        "#
        );
        out
    }

    pub fn gen_entity_struct(
        schema: &Schema,
        entity_type: &String,
        entity_name: &String,
    ) -> String {
        let mut out = String::new();
        // Write derive
        write!(
            out,
            r#"#[derive(Default, Debug, Clone, FromEntity, ToMap)]"#,
        );

        // Write entity name
        write!(
            out,
            r#"
                pub struct {} {{
                    pub id: String,
            "#,
            &entity_name
        );

        match MAPPING_RUST_TYPES_TO_DB.get(entity_type.as_str()) {
            // if it is primitive type
            Some(db_type) => {
                write!(
                    out,
                    r#"pub value: {},"#,
                    MAPPING_DB_TYPES_TO_RUST
                        .get(db_type)
                        .unwrap_or(&Default::default())
                );
            }
            // if it is not primitive type
            None => {
                if let Some(properties) = &schema.properties {
                    for property in properties {
                        let db_type = MAPPING_RUST_TYPES_TO_DB
                            .get(property.data_type.as_str())
                            .unwrap_or(&*DEFAULT_TYPE_DB);
                        match property.array_length {
                            Some(_array_length) => {
                                write!(
                                    out,
                                    r#"pub {}: Vec<{}>,"#,
                                    property.name,
                                    MAPPING_DB_TYPES_TO_RUST
                                        .get(db_type)
                                        .unwrap_or(&Default::default())
                                );
                            }
                            None => {
                                write!(
                                    out,
                                    r#"pub {}: {},"#,
                                    property.name,
                                    MAPPING_DB_TYPES_TO_RUST
                                        .get(db_type)
                                        .unwrap_or(&Default::default())
                                );
                            }
                        }
                    }
                }
            }
        };
        writeln!(
            out,
            r#"
}}"#
        );
        out
    }

    pub fn gen_model_file_function(struct_name: &String) -> String {
        let mut out = String::new();
        // Write entity name
        write!(
            out,
            r#"
                impl Into<Entity> for {struct_name} {{
                    fn into(self) -> Entity {{
                        let map = {struct_name}::to_map(self.clone());
                        Entity::from(map)
                    }}
                }}
                impl {struct_name} {{
                    pub fn save(&self) {{
                        unsafe {{
                            STORE
                                .as_mut()
                                .unwrap()
                                .save("{struct_name}".to_string(), self.clone().into());
                        }}
                    }}
                    pub fn get(entity_id: &String) -> Option<{struct_name}> {{
                        unsafe {{
                            let entity = STORE
                                .as_mut()
                                .unwrap()
                                .get("{struct_name}".to_string(), entity_id);
                            match entity {{
                                Some(e) => Some({struct_name}::from_entity(&e)),
                                None => None,
                            }}
                        }}
                    }}
                    pub fn query(
                        filter: Option<EntityFilter>,
                        order: EntityOrder,
                        range: EntityRange,
                    ) -> Vec<{struct_name}> {{
                        unsafe {{
                            STORE
                                .as_ref()
                                .unwrap()
                                .query("{struct_name}".to_string(), filter, order, range)
                                .iter()
                                .map(|e| {struct_name}::from_entity(e))
                                .collect::<Vec<{struct_name}>>()
                        }}
                    }}
                }}
        "#,
            struct_name = struct_name
        );
        out
    }
}
