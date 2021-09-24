use diesel::debug_query;
use graph::components::store::{
    AttributeNames, EntityCollection, EntityFilter, EntityOrder, EntityRange,
    EntityType,
};
use graph::data::query::QueryExecutionError;
use graph::prelude::q;
use graph::prelude::Logger;
use graph_store_postgres::relational::{Layout, Table};
use graph_store_postgres::relational_queries;
use graph_store_postgres::relational_queries::{
    EntityData, FilterCollection, FilterQuery
};
use inflector::Inflector;
use massbit_common::prelude::diesel::{Connection, PgConnection, RunQueryDsl};
use massbit_common::prelude::serde_json;
use massbit_common::prelude::anyhow;
use std::collections::HashSet;
use std::time::Instant;

/// The name for the primary key column of a table; hardcoded for now
pub(crate) const PRIMARY_KEY_COLUMN: &str = "id";

pub trait TableExt {
    fn gen_relationship(
        &self,
        schema: &str,
    ) -> Result<(Vec<String>, HashSet<String>), anyhow::Error>;
    //fn get_dependencies(&self) -> HashSet<EntityType>;
}
pub trait LayoutExt {
    fn gen_relationship(&self) -> Vec<String>;
    //fn create_dependencies(&self) -> HashMap<EntityType, HashSet<EntityType>>;
    fn create_hasura_tracking_tables(&self) -> (serde_json::Value, serde_json::Value);
    fn create_hasura_tracking_relationships(&self) -> (serde_json::Value, serde_json::Value);
    fn filter<T: relational_queries::FromEntityData>(
        &self,
        logger: &Logger,
        conn: &PgConnection,
        entity_type: EntityType,
        filter: Option<EntityFilter>,
        order: EntityOrder,
        range: EntityRange,
    ) -> Result<Vec<T>, QueryExecutionError>;
}
fn named_type(field_type: &q::Type) -> &str {
    match field_type {
        q::Type::NamedType(name) => name.as_str(),
        q::Type::ListType(child) => named_type(child),
        q::Type::NonNullType(child) => named_type(child),
    }
}
impl TableExt for Table {
    fn gen_relationship(
        &self,
        schema: &str,
    ) -> Result<(Vec<String>, HashSet<String>), anyhow::Error> {
        let mut sqls: Vec<String> = Vec::default();
        let mut references = HashSet::default();
        self.columns
            .iter()
            .filter(|&column| column.is_reference() && !column.is_list())
            .for_each(|column| {
                let reference = named_type(&column.field_type).to_snake_case();
                references.insert(reference.clone());
                sqls.push(format!(
                    r#"alter table {schema}.{table_name}
                    add constraint {table_name}_{reference}_{column_name}_{reference_id}_fk
                    foreign key ("{column_name}", block_range)
                    references {schema}.{reference} ({reference_id}, block_range)"#,
                    schema = schema,
                    table_name = self.name.as_str(),
                    column_name = column.name,
                    reference = reference,
                    reference_id = &PRIMARY_KEY_COLUMN.to_owned()
                ));
            });
        Ok((sqls, references))
    }
    /*
    fn get_dependencies(&self) -> HashSet<EntityType> {
        self.columns
            .iter()
            .filter(|&column| column.is_reference() && !column.is_list())
            .map(|column| EntityType::new(named_type(&column.field_type).to_string()))
            .collect::<HashSet<EntityType>>()
    }
     */
}

impl LayoutExt for Layout {
    fn gen_relationship(&self) -> Vec<String> {
        let mut sqls = Vec::default();
        let mut references = HashSet::new();
        let schema = self.site.namespace.as_str();
        //"create unique index token_id_uindex on sgd0.token (id)";
        self.tables.iter().for_each(|(_key, table)| {
            if let Ok((fks, refs)) = table.gen_relationship(schema) {
                sqls.extend(fks);
                references.extend(refs);
            }
        });
        references.iter().for_each(|r| {
            sqls.insert(
                0,
                format!(
                    r#"create unique index {table}_{field}_block_range_uindex on {schema}.{table} ({field}, block_range)"#,
                    schema = schema,
                    table = r,
                    field = &PRIMARY_KEY_COLUMN.to_owned(),
                ),
            )
        });
        sqls
    }
    /*
    fn create_dependencies(&self) -> HashMap<EntityType, HashSet<EntityType>> {
        let mut dependencies = HashMap::default();
        self.tables.iter().for_each(|(key, table)| {
            let table_deps = table.get_dependencies();
            if table_deps.len() > 0 {
                dependencies.insert(key.cheap_clone(), table_deps);
            }
        });
        dependencies
    }
     */
    fn create_hasura_tracking_tables(&self) -> (serde_json::Value, serde_json::Value) {
        //Generate hasura request to track tables + relationships
        let mut hasura_tables: Vec<serde_json::Value> = Vec::new();
        let mut hasura_down_tables: Vec<serde_json::Value> = Vec::new();
        let schema = self.site.namespace.as_str();
        self.tables.iter().for_each(|(_name, table)| {
            hasura_tables.push(serde_json::json!({
                "type": "track_table",
                "args": {
                    "table": {
                        "schema": schema,
                        "name": table.name.as_str()
                    },
                    "source": "default",
                },
            }));
            hasura_down_tables.push(serde_json::json!({
                "type": "untrack_table",
                "args": {
                    "table" : {
                        "schema": schema,
                        "name": table.name.as_str()
                    },
                    "source": "default",
                    "cascade": true
                },
            }));
        });
        //hasura_tables.append(&mut hasura_relations);
        //hasura_down_relations.append(&mut hasura_down_tables);
        (
            serde_json::json!({
                "type": "bulk",
                "args" : hasura_tables
            }),
            serde_json::json!({
                "type": "bulk",
                "args" : hasura_down_tables
            }),
        )
    }
    fn create_hasura_tracking_relationships(&self) -> (serde_json::Value, serde_json::Value) {
        let mut hasura_relations: Vec<serde_json::Value> = Vec::new();
        let mut hasura_down_relations: Vec<serde_json::Value> = Vec::new();
        let schema = self.site.namespace.as_str();
        self.tables.iter().for_each(|(_name, table)| {
            table
                .columns
                .iter()
                .filter(|col| col.is_reference() && !col.is_list())
                .for_each(|column| {
                    let field_type = named_type(&column.field_type);
                    let reference = field_type.to_snake_case();
                    // This is be a unique identifier to avoid the problem: An entity can have multiple reference to another entity.
                    // Example: Pair Entity (token0: Token!, token1: Token!)
                    let rel_name = format!("{}_{}", field_type, column.name.as_str());
                    // Don't create relationship for child table because if it's type is array the parent already has the foreign key constraint (I think)
                    hasura_relations.push(serde_json::json!({
                       "type": "create_object_relationship",
                       "args": {
                           "table": {
                               "name": table.name.as_str(),
                               "schema": schema
                           },
                           "name": rel_name.as_str(),
                           "using" : {
                                "manual_configuration":{
                                    "remote_table":{
                                        "name":reference,
                                        "schema": schema
                                    },
                                    "column_mapping":{
                                        column.name.as_str(): PRIMARY_KEY_COLUMN,
                                    }
                                }
                           }
                       }
                    }));
                    hasura_down_relations.push(serde_json::json!({
                        "type": "drop_relationship",
                        "args": {
                            "relationship": rel_name,
                            "source": "default",
                            "table": {
                                "schema": schema,
                                "name": table.name.as_str()
                            }
                        }
                    }));
                    let ref_table = named_type(&column.field_type).to_snake_case();
                    // This is be a unique identifier to avoid the problem: An entity can have multiple reference to another entity.
                    //Example: Pair Entity (token0: Token!, token1: Token!)
                    let rel_name = format!("{}_{}", table.name.as_str(), column.name.as_str());
                    // Don't create relationship for child table because if it's type is array the parent already has the foreign key constraint (I think)
                    hasura_relations.push(serde_json::json!({
                        "type": "create_array_relationship",
                        "args": {
                            "name": rel_name.as_str(),
                            "table": {
                                "name": ref_table.clone(),
                                "schema": schema,
                            },
                            "using" : {
                                "manual_configuration":{
                                    "remote_table":{
                                        "name": table.name.as_str(),
                                        "schema": schema
                                    },
                                    "source":"default",
                                    "column_mapping":{
                                        "id":column.name.as_str(),
                                    }
                                }
                            }
                        }
                    }));

                    hasura_down_relations.push(serde_json::json!({
                        "type": "drop_relationship",
                        "args": {
                            "relationship": rel_name,
                            "source": "default",
                            "table": {
                                "name": ref_table,
                                "schema": schema,
                            },
                         }
                    }));
                });
        });
        (
            serde_json::json!({
                "type": "bulk",
                "args" : hasura_relations
            }),
            serde_json::json!({
                "type": "bulk",
                "args" : hasura_down_relations
            }),
        )

        /*
        //tracking relation ship with table relationship
           table
           .columns
           .iter()
           .filter(|col| col.is_reference() && !col.is_list())
           .for_each(|column| {
               // Don't create relationship for child table because if it's type is array the parent already has the foreign key constraint (I think)
               hasura_relations.push(serde_json::json!({
                   "type": "create_object_relationship",
                   "args": {
                       "table": {
                           "name": table.name.as_str(),
                           "schema": schema
                       },
                       "name": format!("{}_{}",named_type(&column.field_type),column.name.as_str()), // This is be a unique identifier to avoid the problem: An entity can have multiple reference to another entity. Example: Pair Entity (token0: Token!, token1: Token!)
                       "using" : {
                           "foreign_key_constraint_on" : column.name.as_str()
                       }
                   }
               }));

               hasura_down_relations.push(serde_json::json!({
                   "type": "drop_relationship",
                   "args": {
                       "relationship": named_type(&column.field_type),
                       "source": "default",
                       "table": {
                           "schema": schema,
                           "name": table.name.as_str()
                       }
                   }
               }));
               let ref_table = named_type(&column.field_type).to_snake_case();

               // Don't create relationship for child table because if it's type is array the parent already has the foreign key constraint (I think)
               hasura_relations.push(serde_json::json!({
                   "type": "create_array_relationship",
                   "args": {
                       "name": format!("{}_{}",table.name.as_str(),column.name.as_str()), // This is be a unique identifier to avoid the problem: An entity can have multiple reference to another entity. Example: Pair Entity (token0: Token!, token1: Token!)
                       "table": {
                           "name": ref_table.clone(),
                           "schema": schema,
                       },
                       "using" : {
                           "foreign_key_constraint_on" : {
                               "table": {
                                   "name": table.name.as_str(),
                                   "schema": schema
                               },
                               "column": column.name.as_str()
                           }
                       }
                   }
               }));

               hasura_down_relations.push(serde_json::json!({
                   "type": "drop_relationship",
                   "args": {
                       "relationship": table.name.as_str(),
                       "source": "default",
                       "table": {
                           "name": ref_table,
                           "schema": schema,
                       },
                    }
               }));
           });
        */
    }

    /// order is a tuple (attribute, value_type, direction)
    fn filter<T: relational_queries::FromEntityData>(
        &self,
        logger: &Logger,
        conn: &PgConnection,
        entity_type: EntityType,
        filter: Option<EntityFilter>,
        order: EntityOrder,
        range: EntityRange,
    ) -> Result<Vec<T>, QueryExecutionError> {
        // fn log_query_timing(
        //     logger: &Logger,
        //     query: &FilterQuery,
        //     elapsed: Duration,
        //     entity_count: usize,
        // ) {
        //     // 20kB
        //     const MAXLEN: usize = 20_480;
        //
        //     if !*graph::log::LOG_SQL_TIMING {
        //         return;
        //     }
        //
        //     let mut text = debug_query(&query).to_string().replace("\n", "\t");
        //     // If the query + bind variables is more than MAXLEN, truncate it;
        //     // this will happen when queries have very large bind variables
        //     // (e.g., long arrays of string ids)
        //     if text.len() > MAXLEN {
        //         text.truncate(MAXLEN);
        //         text.push_str(" ...");
        //     }
        //     slog::info!(
        //         logger,
        //         "Query timing (SQL)";
        //         "query" => text,
        //         "time_ms" => elapsed.as_millis(),
        //         "entity_count" => entity_count
        //     );
        // }

        let collection = EntityCollection::All(vec![(entity_type, AttributeNames::All)]);
        let filter_collection = FilterCollection::new(&self, collection, filter.as_ref())?;
        let query = FilterQuery::new(
            &filter_collection,
            filter.as_ref(),
            order,
            range,
            std::i32::MAX, //Ignore blockrange
            None,
        )?;
        let query_clone = query.clone();
        let values = conn
            .transaction(|| {
                // if let Some(ref timeout_sql) = *STATEMENT_TIMEOUT {
                //     conn.batch_execute(timeout_sql)?;
                // }
                query.load::<EntityData>(conn)
            })
            .map_err(|e| {
                QueryExecutionError::ResolveEntitiesError(format!(
                    "{}, query = {:?}",
                    e,
                    debug_query(&query_clone).to_string()
                ))
            })?;
        //log_query_timing(logger, &query_clone, start.elapsed(), values.len());
        values
            .into_iter()
            .map(|entity_data| {
                entity_data
                    .deserialize_with_layout(self)
                    .map_err(|e| e.into())
            })
            .collect()
    }
}
