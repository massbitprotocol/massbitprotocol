use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::sql_types::BigInt;
use graph::cheap_clone::CheapClone;
use graph::components::metrics::stopwatch::StopwatchMetrics;
use graph::components::store::{BlockNumber, EntityKey, EntityType};
use graph::components::subgraph::Entity;
use graph::prelude::{q, StoreError};
use graph_store_postgres::relational::{Layout, Table};
use graph_store_postgres::relational_queries::ClampRangeQuery;
use inflector::Inflector;
use massbit_common::prelude::serde_json;
use massbit_common::prelude::serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

/// The name for the primary key column of a table; hardcoded for now
pub(crate) const PRIMARY_KEY_COLUMN: &str = "id";

pub trait TableExt {
    fn gen_relationship(
        &self,
        schema: &str,
    ) -> Result<(Vec<String>, HashSet<String>), anyhow::Error>;
    fn get_dependencies(&self, layout: &Layout) -> Vec<EntityType>;
}
pub trait LayoutExt {
    fn gen_relationship(&self) -> Vec<String>;
    fn create_dependencies(&self) -> HashMap<EntityType, Vec<EntityType>>;
    fn create_hasura_payloads(&self) -> (serde_json::Value, serde_json::Value);
    fn simple_update(
        &self,
        conn: &PgConnection,
        entity_type: &EntityType,
        entities: &mut [(EntityKey, Entity)],
        block: BlockNumber,
        stopwatch: &StopwatchMetrics,
    ) -> Result<usize, StoreError>;
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
                add constraint {table_name}_{column_name}_{reference}_{reference_id}_fk
                foreign key ("{column_name}")
                references {schema}.{reference} ({reference_id})"#,
                    schema = schema,
                    table_name = self.name.as_str(),
                    column_name = column.name,
                    reference = reference,
                    reference_id = &PRIMARY_KEY_COLUMN.to_owned()
                ));
            });
        Ok((sqls, references))
    }

    fn get_dependencies(&self, layout: &Layout) -> Vec<EntityType> {
        self.columns
            .iter()
            .filter(|&column| column.is_reference() && !column.is_list())
            .map(|column| EntityType::new(named_type(&column.field_type).to_string()))
            .collect::<Vec<EntityType>>()
    }
}

impl LayoutExt for Layout {
    fn gen_relationship(&self) -> Vec<String> {
        let mut sqls = Vec::default();
        let mut references = HashSet::new();
        let schema = self.site.namespace.as_str();
        //"create unique index token_id_uindex on sgd0.token (id)";
        self.tables.iter().for_each(|(key, table)| {
            if let Ok((mut fks, mut refs)) = table.gen_relationship(schema) {
                sqls.extend(fks);
                references.extend(refs);
            }
        });
        references.iter().for_each(|r| {
            sqls.insert(
                0,
                format!(
                    r#"create unique index {table}_{field}_uindex on {schema}.{table} ({field})"#,
                    schema = schema,
                    table = r,
                    field = &PRIMARY_KEY_COLUMN.to_owned(),
                ),
            )
        });
        sqls
    }
    fn create_dependencies(&self) -> HashMap<EntityType, Vec<EntityType>> {
        let mut dependencies = HashMap::default();
        self.tables.iter().for_each(|(key, table)| {
            dependencies.insert(key.cheap_clone(), table.get_dependencies(&self));
        });
        dependencies
    }
    fn simple_update(
        &self,
        conn: &PgConnection,
        entity_type: &EntityType,
        entities: &mut [(EntityKey, Entity)],
        block: BlockNumber,
        stopwatch: &StopwatchMetrics,
    ) -> Result<usize, StoreError> {
        let table = self.table_for_entity(&entity_type)?;
        let entity_keys: Vec<&str> = entities
            .iter()
            .map(|(key, _)| key.entity_id.as_str())
            .collect();

        let section = stopwatch.start_section("update_modification_clamp_range_query");
        let result =
            ClampRangeQuery::new(table, &entity_type, &entity_keys, block).execute(conn)?;
        section.end();
        /*
        let _section = stopwatch.start_section("update_modification_insert_query");
        let mut count = 0;

        // Each operation must respect the maximum number of bindings allowed in PostgreSQL queries,
        // so we need to act in chunks whose size is defined by the number of entities times the
        // number of attributes each entity type has.
        // We add 1 to account for the `block_range` bind parameter
        let chunk_size = POSTGRES_MAX_PARAMETERS / (table.columns.len() + 1);
        for chunk in entities.chunks_mut(chunk_size) {
            count += InsertQuery::new(table, chunk, block)?.execute(conn)?;
        }
         */
        Ok(result)
    }
    fn create_hasura_payloads(&self) -> (Value, Value) {
        //Generate hasura request to track tables + relationships
        let mut hasura_tables: Vec<serde_json::Value> = Vec::new();
        let mut hasura_relations: Vec<serde_json::Value> = Vec::new();
        let mut hasura_down_relations: Vec<serde_json::Value> = Vec::new();
        let mut hasura_down_tables: Vec<serde_json::Value> = Vec::new();
        let schema = self.site.namespace.as_str();
        self.tables.iter().for_each(|(name, table)| {
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
            /*
             * 21-07-27
             * vuviettai: hasura use create_object_relationship api to create relationship in DB
             * Migration sql already include this creation.
             */
            /*
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
                            },                        }
                    }));
                });
             */
        });
        hasura_tables.append(&mut hasura_relations);
        hasura_down_relations.append(&mut hasura_down_tables);
        (
            serde_json::json!({
                "type": "bulk",
                "args" : hasura_tables
            }),
            serde_json::json!({
                "type": "bulk",
                "args" : hasura_down_relations
            }),
        )
    }
}
