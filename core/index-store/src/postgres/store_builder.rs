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
/*
use crate::store::postgres::ConnectionPool;
 */
use super::relational::LayoutExt;
use super::PostgresIndexStore;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel::QueryableByName;
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

lazy_static! {
    pub static ref GRAPH_NODE: NodeId = NodeId::new("graph_node").unwrap();
    pub static ref NAMESPACE: Namespace = Namespace::new("sgd0".to_string()).unwrap();
    pub static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    pub static ref DEPLOYMENT_HASH: DeploymentHash = DeploymentHash::new("_indexer").unwrap();
    pub static ref NETWORK: String = String::from("");
}

const CONN_POOL_SIZE: u32 = 20;

pub struct StoreBuilder {}
impl StoreBuilder {
    pub fn create_store<'a, P: AsRef<Path>>(
        _indexer: &str,
        schema_path: P,
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
        path: P,
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
        /*
        let site = make_dummy_site(
            DEPLOYMENT_HASH.cheap_clone(),
            NAMESPACE.clone(),
            NETWORK.clone(),
        );
         */
        //Create simple site
        let site = Site {
            id: DeploymentId(0),
            deployment: DEPLOYMENT_HASH.cheap_clone(),
            shard: PRIMARY_SHARD.clone(),
            namespace: NAMESPACE.clone(),
            network: NETWORK.clone(),
            active: true,
            _creation_disallowed: (),
        };

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
                tokio::spawn(async move {
                    let payload = serde_json::json!({
                        "type": "bulk",
                        "args" : vec![track_tables, track_relationships]
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

#[derive(Debug, Clone, QueryableByName)]
struct Counter {
    #[sql_type = "BigInt"]
    pub count: i64,
}
