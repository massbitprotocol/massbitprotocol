//use super::relational::Layout;
use crate::prelude::{Arc, Logger};
use graph_mock::MockMetricsRegistry;
use graph_store_postgres::{
    command_support::{
        catalog::{self, Connection, Site},
        Namespace,
    },
    connection_pool::ConnectionPool,
    relational,
    relational::{Layout, Table},
    PRIMARY_SHARD,
};
/*
use crate::store::postgres::ConnectionPool;
 */
use crate::store::PostgresIndexStore;
use anyhow::anyhow;
use core::ops::{Deref, DerefMut};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::sql_types::BigInt;
use diesel::QueryableByName;
use graph::cheap_clone::CheapClone;
use graph::components::metrics::MetricsRegistry;
use graph::data::schema::Schema;
use graph::log::logger;
use graph::prelude::{q, DeploymentHash, NodeId, StoreError};
use graph::util::security::SafeDisplay;
use graph_node::{
    config::{Config, Opt, PoolSize, Shard as ShardConfig},
    store_builder::StoreBuilder as GraphStoreBuilder,
};
use graph_store_postgres::command_support::Catalog;
use graph_store_postgres::layout_for_tests::make_dummy_site;
use graph_store_postgres::subgraph_store::SubgraphStoreInner;
use graph_store_postgres::{deployment, SubgraphStore};
use inflector::Inflector;
use massbit_common::consts::HASURA_URL;
use massbit_common::prelude::diesel::connection::SimpleConnection;
use massbit_common::prelude::diesel::{sql_query, RunQueryDsl};
use massbit_common::prelude::lazy_static::lazy_static;
use massbit_common::prelude::log::{self, debug, error, info};
use massbit_common::prelude::reqwest::Client;
use massbit_common::prelude::serde_json;
use massbit_common::prelude::serde_json::Value;
use massbit_common::prelude::tokio_compat_02::FutureExt;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

lazy_static! {
    pub static ref GRAPH_NODE: NodeId = NodeId::new("graph_node").unwrap();
    pub static ref NAMESPACE: Namespace = Namespace::new("sgd0".to_string()).unwrap();
    pub static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    pub static ref DEPLOYMENT_HASH: DeploymentHash = DeploymentHash::new("_indexer").unwrap();
    pub static ref NETWORK: String = String::from("");
}

const CONN_POOL_SIZE: u32 = 20;
/// The name for the primary key column of a table; hardcoded for now
pub(crate) const PRIMARY_KEY_COLUMN: &str = "id";
pub struct StoreBuilder {}
impl StoreBuilder {
    pub fn create_store<'a, P: AsRef<Path>>(
        indexer: &str,
        schema_path: &P,
    ) -> Result<PostgresIndexStore, anyhow::Error> {
        let logger = logger(false);
        let mut opt = Opt::default();
        opt.postgres_url = Some(DATABASE_CONNECTION_STRING.clone());
        opt.store_connection_pool_size = CONN_POOL_SIZE;

        let config = Config::load(&logger, &opt).expect("config is not valid");
        let registry = Arc::new(MockMetricsRegistry::new());
        let shard_config = config.stores.get(PRIMARY_SHARD.as_str()).unwrap();
        let shard_name = String::from(PRIMARY_SHARD.as_str());
        /*
        let shard_config = ShardConfig {
            connection: DATABASE_CONNECTION_STRING.clone(),
            weight: 0,
            pool_size: PoolSize::Fixed(CONN_POOL_SIZE),
            fdw_pool_size: Default::default(),
            replicas: Default::default(),
        };
         */
        /*
        let (store, pools) = GraphStoreBuilder::make_subgraph_store_and_pools(
            &logger,
            &GRAPH_NODE,
            &config,
            registry.cheap_clone(),
        );
        let store = GraphStoreBuilder::make_subgraph_store(
            &logger,
            &GRAPH_NODE,
            &config,
            registry.cheap_clone(),
        );
         */
        let connection = GraphStoreBuilder::main_pool(
            &logger,
            &GRAPH_NODE,
            &shard_name,
            &shard_config,
            registry.cheap_clone(),
        );
        match Self::create_relational_schema(schema_path, &connection) {
            Ok(layout) => {
                //let entity_dependencies = layout.create_dependencies();
                Ok(PostgresIndexStore {
                    connection,
                    layout,
                    //entity_dependencies,
                    logger,
                })
            }
            Err(e) => Err(e.into()),
        }
    }
    pub fn create_relational_schema<P: AsRef<Path>>(
        path: &P,
        connection: &ConnectionPool,
    ) -> Result<Layout, StoreError> {
        let mut schema_buffer = String::new();
        let mut file = File::open(path).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
        file.read_to_string(&mut schema_buffer)
            .expect("Unable to read string");
        //let deployment_hash = DeploymentHash::new(indexer_hash.to_string()).unwrap();
        let deployment_hash = DeploymentHash::new("_indexer").unwrap();
        let schema = Schema::parse(schema_buffer.as_str(), deployment_hash.cheap_clone()).unwrap();

        let logger = Logger::root(slog::Discard, slog::o!());
        let conn = connection.get_with_timeout_warning(&logger).unwrap();
        /*
        let site = Connection::new(&conn)
            .allocate_site(PRIMARY_SHARD.clone(), &DEPLOYMENT_HASH, NETWORK.clone())
            .unwrap();
        */
        let site = make_dummy_site(
            DEPLOYMENT_HASH.cheap_clone(),
            NAMESPACE.clone(),
            NETWORK.clone(),
        );
        let result = sql_query(format!(
            "SELECT count(schema_name) FROM information_schema.schemata WHERE schema_name = '{}'",
            NAMESPACE.as_str()
        ))
        //.bind::<Text, _>(NAMESPACE.as_str())
        .get_results::<Counter>(&conn)
        .expect("Query failed")
        .pop()
        .expect("No record found");

        //let exists = deployment::exists(&conn, &site)?;
        let arc_site = Arc::new(site);
        let catalog = Catalog::make_empty(arc_site.clone()).unwrap();
        //let catalog = Catalog::new(&conn.deref(), arc_site.clone())?;
        let conn = connection.get_with_timeout_warning(&logger)?;
        if result.count == 0 {
            //Create schema
            match sql_query(format!("create schema {}", NAMESPACE.as_str())).execute(&conn) {
                Ok(_) => {}
                Err(err) => {
                    error!("Error while create schema {:?}", err)
                }
            };
            //Need execute command CREATE EXTENSION btree_gist; on db
        }
        match Layout::new(arc_site, &schema, catalog, false) {
            Ok(layout) => {
                let sql = layout.as_ddl().map_err(|_| {
                    StoreError::Unknown(anyhow!("failed to generate DDL for layout"))
                })?;
                let relationships = layout.gen_relationship();
                let (hasura_up, _) = layout.create_hasura_payloads();
                tokio::spawn(async move {
                    match conn.batch_execute(&sql) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("{:?}", e);
                        }
                    }
                    if relationships.len() > 0 {
                        let query = relationships.join(";");
                        log::info!("Create relationships: {:?}", &query);
                        match conn.batch_execute(&query) {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("Error while crate relation {:?}", err)
                            }
                        }
                    }
                    let response = Client::new()
                        .post(&*HASURA_URL)
                        .json(&hasura_up)
                        .send()
                        .compat()
                        .await
                        .unwrap();
                    log::info!("Hasura {:?}", response);
                });
                Ok(layout)
            }
            Err(e) => Err(e),
        }

        /*
        let result = Layout::create_relational_schema(&conn.deref(), arc_site, &schema);
        match result {
            Ok(layout) => {
                Self::create_relationships(&layout, &conn.deref());
                let (hasura_up, _) = layout.create_hasura_payloads();
                //println!("{:?}", serde_json::to_string(&hasura_up).unwrap());
                tokio::spawn(async move {
                    let response = Client::new()
                        .post(&*HASURA_URL)
                        .json(&hasura_up)
                        .send()
                        .compat()
                        .await
                        .unwrap();
                    log::info!("Hasura {:?}", response);
                });
                Ok(layout)
            }
            Err(e) => Err(e),
        }
        */
    }

    pub fn create_relationships(layout: &Layout, connection: &PgConnection) {
        let relationships = layout.gen_relationship();
        if relationships.len() > 0 {
            let query = relationships.join(";");
            log::info!("Create relationships: {:?}", &query);
            match connection.batch_execute(&query) {
                Ok(_) => {}
                Err(err) => {
                    println!("Error while crate relation {:?}", err);
                }
            }
        }
    }
}
fn named_type(field_type: &q::Type) -> &str {
    match field_type {
        q::Type::NamedType(name) => name.as_str(),
        q::Type::ListType(child) => named_type(child),
        q::Type::NonNullType(child) => named_type(child),
    }
}

#[derive(Debug, Clone, QueryableByName)]
struct Counter {
    #[sql_type = "BigInt"]
    pub count: i64,
}

pub trait TableExt {
    fn gen_relationship(
        &self,
        schema: &str,
    ) -> Result<(Vec<String>, HashSet<String>), anyhow::Error>;
    fn get_dependencies(&self, layout: &Layout) -> Vec<String>;
}
pub trait LayoutExt {
    fn gen_relationship(&self) -> Vec<String>;
    fn create_dependencies(&self) -> HashMap<String, Vec<String>>;
    fn create_hasura_payloads(&self) -> (serde_json::Value, serde_json::Value);
}
pub trait EntityDependencies {
    fn create_dependencies(&self) -> HashMap<String, Vec<String>>;
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

    fn get_dependencies(&self, layout: &Layout) -> Vec<String> {
        let mut dependencies = Vec::default();

        dependencies
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
    fn create_dependencies(&self) -> HashMap<String, Vec<String>> {
        let mut dependencies = HashMap::default();
        self.tables.iter().for_each(|(key, table)| {
            dependencies.insert(
                String::from(table.name.as_str()),
                table.get_dependencies(&self),
            );
        });
        dependencies
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
