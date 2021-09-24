use graph_mock::MockMetricsRegistry;
use graph_store_postgres::{
    command_support::{catalog::Site, Namespace},
    connection_pool::ConnectionPool,
    relational::Layout,
    PRIMARY_SHARD,
};
use massbit_common::prelude::{
    anyhow::{self, anyhow},
    slog::{self, Logger},
};
use std::sync::Arc;
use super::relational::LayoutExt;
use super::PostgresIndexStore;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel::QueryableByName;
use diesel_migrations::embed_migrations;
use graph::cheap_clone::CheapClone;
use graph::data::schema::Schema;
use graph::log::logger;
use graph::prelude::{DeploymentHash, NodeId, StoreError};
use graph_node::{
    config::{Config, Opt},
    store_builder::StoreBuilder as GraphStoreBuilder,
};
use graph_store_postgres::command_support::Catalog;
use graph_store_postgres::primary::DeploymentId;
use massbit_common::consts::HASURA_URL;
use massbit_common::prelude::diesel::connection::SimpleConnection;
use massbit_common::prelude::diesel::{sql_query, RunQueryDsl};
use massbit_common::prelude::lazy_static::lazy_static;
use massbit_common::prelude::log::{self, error};
use massbit_common::prelude::reqwest::Client;
use massbit_common::prelude::serde_json;
use massbit_common::prelude::tokio_compat_02::FutureExt;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use crate::schema::indexers;
use crate::models::Indexer;
lazy_static! {
    pub static ref GRAPH_NODE: NodeId = NodeId::new("graph_node").unwrap();
    //pub static ref NAMESPACE: Namespace = Namespace::new("sgd0".to_string()).unwrap();
    pub static ref DEPLOYMENT_HASH: DeploymentHash = DeploymentHash::new("_indexer").unwrap();
    pub static ref NETWORK: String = String::from("");
}

const CONN_POOL_SIZE: u32 = 20;
//embed_migrations!("./migrations");

pub struct StoreBuilder {}
impl StoreBuilder {
    pub fn prepare_schema(indexer_hash: &str, conn: &PgConnection) -> Result<String, anyhow::Error>  {
        log::info!("Prepare schema for indexer {}", indexer_hash);
        let results = indexers::table.filter(indexers::id.eq(indexer_hash))
            .limit(1)
            .load::<Indexer>(conn)
            .expect("Error loading indexer state");
        let schema_name : String = match results.get(0) {
            Some(val) => {
                if val.schema_name == "" {
                    println!("Update schema name");
                    let schema_name = format!("sgd{}", val.v_id);
                    diesel::update(indexers::table.filter(indexers::v_id.eq(val.v_id)))
                        .set(indexers::schema_name.eq(&schema_name))
                        .execute(conn);
                    schema_name
                } else {
                    val.schema_name.clone()
                }
            },
            None => {
                let state = diesel::insert_into(indexers::table)
                    .values(&(indexers::id.eq(indexer_hash),
                             indexers::schema_name.eq(""),
                             indexers::got_block.eq(0_i64))
                    )
                    //.execute(conn);
                    .get_result::<Indexer>(conn)?;
                let schema_name = format!("sgd{}", state.v_id);
                diesel::update(indexers::table.filter(indexers::v_id.eq(state.v_id)))
                    .set(indexers::schema_name.eq(&schema_name))
                    .execute(conn);
                schema_name
            }
        };

        let counter = sql_query(format!(
            "SELECT count(schema_name) FROM information_schema.schemata WHERE schema_name = '{}'",
            schema_name.as_str()
        ))
            .get_results::<Counter>(conn)
            .expect("Query failed")
            .pop()
            .expect("No record found");
        if counter.count == 0 {
            //Create schema
            match sql_query(format!("create schema {}", schema_name.as_str())).execute(conn) {
                Ok(_) => {}
                Err(err) => {
                    error!("Error while create schema {:?}", err)
                }
            };
            //Need execute command CREATE EXTENSION btree_gist; on db
        }

        Ok(schema_name)
    }
    pub fn create_store<P: AsRef<Path>>(
        indexer: &str,
        schema_path: P,
    ) -> Result<PostgresIndexStore, anyhow::Error> {
        let logger = logger(false);
        let mut opt = Opt::default();
        opt.postgres_url = Some(crate::DATABASE_CONNECTION_STRING.clone());
        opt.store_connection_pool_size = CONN_POOL_SIZE;

        let config = Config::load(&logger, &opt).expect("config is not valid");
        let registry = Arc::new(MockMetricsRegistry::new());
        let shard_config = config.stores.get(PRIMARY_SHARD.as_str()).unwrap();
        let shard_name = String::from(PRIMARY_SHARD.as_str());
        let connection = GraphStoreBuilder::main_pool(
            &logger,
            &GRAPH_NODE,
            &shard_name,
            &shard_config,
            registry.cheap_clone(),
            Arc::new(vec![]),
        );
        //Skip run migration in connection_pool
        connection.skip_setup();
        let logger = Logger::root(slog::Discard, slog::o!());
        let conn = connection.get_with_timeout_warning(&logger).unwrap();
        // match embedded_migrations::run(&conn) {
        //     Ok(res) => println!("Finished embedded_migration {:?}", &res),
        //     Err(err) => println!("{:?}", &err)
        // };
        match StoreBuilder::prepare_schema(indexer, &conn) {
            Ok(schema) => {
                match Self::create_relational_schema(schema_path, schema, &connection) {
                    Ok(layout) => {
                        //let entity_dependencies = layout.create_dependencies();
                        Ok(PostgresIndexStore {
                            connection,
                            layout,
                            logger
                        })
                    }
                    Err(e) => Err(e.into()),
                }
            }
            Err(err) => Err(err)
        }
    }
    pub fn create_relational_schema<P: AsRef<Path>>(
        path: P,
        schema_name: String,
        connection: &ConnectionPool,
    ) -> Result<Layout, StoreError> {
        let mut schema_buffer = String::new();
        let mut file = File::open(path).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
        file.read_to_string(&mut schema_buffer)
            .expect("Unable to read string");
        //let deployment_hash = DeploymentHash::new(indexer_hash.to_string()).unwrap();
        let deployment_hash = DeploymentHash::new("_indexer").unwrap();
        let schema = Schema::parse(schema_buffer.as_str(), deployment_hash.cheap_clone()).unwrap();
        let namespace = Namespace::new(schema_name).unwrap();
        let logger = Logger::root(slog::Discard, slog::o!());
        let conn = connection.get_with_timeout_warning(&logger).unwrap();
        //Create simple site
        let site = Site {
            id: DeploymentId(0),
            deployment: DEPLOYMENT_HASH.cheap_clone(),
            shard: PRIMARY_SHARD.clone(),
            namespace,
            network: NETWORK.clone(),
            active: true,
            _creation_disallowed: (),
        };

        let arc_site = Arc::new(site);
        let catalog = Catalog::make_empty(arc_site.clone()).unwrap();
        match Layout::new(arc_site, &schema, catalog, false) {
            Ok(layout) => {
                let sql = layout.as_ddl().map_err(|_| {
                    StoreError::Unknown(anyhow!("failed to generate DDL for layout"))
                })?;
                //let sql_relationships = layout.gen_relationship();
                match conn.batch_execute(&sql) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{:?}", e);
                    }
                }
                /*
                if sql_relationships.len() > 0 {
                    let query = sql_relationships.join(";");
                    log::info!("Create relationships: {:?}", &query);
                    match conn.batch_execute(&query) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Error while crate relation {:?}", err)
                        }
                    }
                }
                 */

                let (track_tables, _) = layout.create_hasura_tracking_tables();
                let (track_relationships, _) = layout.create_hasura_tracking_relationships();
                let reload_metadata = serde_json::json!({
                    "type": "reload_metadata",
                    "args": {
                        "reload_remote_schemas": true,
                    },
                });
                tokio::spawn(async move {
                    let payload = serde_json::json!({
                        "type": "bulk",
                        "args" : vec![track_tables, track_relationships, reload_metadata]
                    });
                    let response = Client::new()
                        .post(&*HASURA_URL)
                        .json(&payload)
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

#[derive(Debug, Clone, QueryableByName)]
struct Counter {
    #[sql_type = "BigInt"]
    pub count: i64,
}
