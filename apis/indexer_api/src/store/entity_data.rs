use crate::store::converter::{FromColumnValue, FromEntityData};
use diesel::sql_types::{Jsonb, Text};
use massbit::components::store::EntityType;
use massbit::prelude::StoreError;
use massbit_common::prelude::diesel::{
    result::Error as DieselError, ExpressionMethods, QueryResult,
};
use massbit_common::prelude::serde_json;
use massbit_solana_sdk::scalar;
use massbit_store_postgres::relational::{ColumnType, Layout, SqlName};
use std::str::FromStr;
//use massbit_store_postgres::relational_queries::EntityData;

/// Helper struct for retrieving entities from the database. With diesel, we
/// can only run queries that return columns whose number and type are known
/// at compile time. Because of that, we retrieve the actual data for an
/// entity as Jsonb by converting the row containing the entity using the
/// `to_jsonb` function.
#[derive(QueryableByName)]
pub struct EntityData {
    #[sql_type = "Text"]
    pub entity: String,
    #[sql_type = "Jsonb"]
    pub data: serde_json::Value,
}

pub trait EntityDataTrait {
    fn to_entity<T: FromEntityData>(self, layout: &Layout) -> Result<T, StoreError>;
}
impl EntityData {
    pub fn entity_type(&self) -> EntityType {
        EntityType::new(self.entity.clone())
    }

    /// Map the `EntityData` using the schema information in `Layout`
    pub fn deserialize_with_layout<T: FromEntityData>(
        self,
        layout: &Layout,
    ) -> Result<T, StoreError> {
        let entity_type = EntityType::new(self.entity);
        let table = layout.table_for_entity(&entity_type)?;

        use serde_json::Value as j;
        match self.data {
            j::Object(map) => {
                let mut out = T::default();
                out.insert_entity_data(
                    "__typename".to_owned(),
                    T::Value::from_string(entity_type.into_string()),
                );
                for (key, json) in map {
                    // Simply ignore keys that do not have an underlying table
                    // column; those will be things like the block_range that
                    // is used internally for versioning
                    if key == "g$parent_id" {
                        let value = T::Value::from_column_value(&ColumnType::String, json)?;
                        out.insert_entity_data("g$parent_id".to_owned(), value);
                    } else if let Some(column) = table.column(&SqlName::verbatim(key)) {
                        let value = T::Value::from_column_value(&column.column_type, json)?;
                        if !value.is_null() {
                            out.insert_entity_data(column.field.clone(), value);
                        }
                    }
                }
                Ok(out)
            }
            _ => unreachable!(
                "we use `to_json` in our queries, and will therefore always get an object back"
            ),
        }
    }
}

fn str_as_bytes(id: &str) -> QueryResult<scalar::Bytes> {
    scalar::Bytes::from_str(&id).map_err(|e| DieselError::SerializationError(Box::new(e)))
}

/// Convert Postgres string representation of bytes "\xdeadbeef"
/// to ours of just "deadbeef".
fn bytes_as_str(id: &str) -> String {
    id.trim_start_matches("\\x").to_owned()
}

// impl EntityData {
//     /// Map the `EntityData` using the schema information in `Layout`
//     pub fn to_entity<T: FromEntityData>(self, layout: &Layout) -> Result<T, StoreError> {
//         let entity_type = EntityType::new(self.entity);
//         let table = layout.table_for_entity(&entity_type)?;
//
//         use serde_json::Value as j;
//         match self.data {
//             j::Object(map) => {
//                 let mut out = T::default();
//                 out.insert_entity_data(
//                     "__typename".to_owned(),
//                     T::Value::from_string(entity_type.into_string()),
//                 );
//                 for (key, json) in map {
//                     // Simply ignore keys that do not have an underlying table
//                     // column; those will be things like the block_range that
//                     // is used internally for versioning
//                     if key == "g$parent_id" {
//                         let value = T::Value::from_column_value(&ColumnType::String, json)?;
//                         out.insert_entity_data("g$parent_id".to_owned(), value);
//                     } else if let Some(column) = table.column(&SqlName::verbatim(key)) {
//                         let value = T::Value::from_column_value(&column.column_type, json)?;
//                         if !value.is_null() {
//                             out.insert_entity_data(column.field.clone(), value);
//                         }
//                     }
//                 }
//                 Ok(out)
//             }
//             _ => unreachable!(
//                 "we use `to_json` in our queries, and will therefore always get an object back"
//             ),
//         }
//     }
// }
