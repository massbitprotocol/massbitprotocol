//use super::relational::Layout;
use crate::mock::MockMetricsRegistry;
use crate::prelude::{Arc, Logger};
use graph_store_postgres::{
    command_support::{
        catalog::{self, Connection, Site},
        Namespace,
    },
    connection_pool::ConnectionPool,
    relational::{Layout, Table},
    PRIMARY_SHARD,
};
/*
use crate::store::postgres::ConnectionPool;
 */
use crate::store::PostgresIndexStore;
use core::ops::{Deref, DerefMut};
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
use massbit_common::prelude::diesel::connection::SimpleConnection;
use massbit_common::prelude::diesel::sql_types::Text;
use massbit_common::prelude::diesel::{sql_query, RunQueryDsl};
use massbit_common::prelude::lazy_static::lazy_static;
use massbit_common::prelude::tokio_postgres::Row;
use slog::{info, o};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

lazy_static! {
    static ref GRAPH_NODE: NodeId = NodeId::new("graph_node").unwrap();
    static ref NAMESPACE: Namespace = Namespace::new("sgd0".to_string()).unwrap();
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    static ref DEPLOYMENT_HASH: DeploymentHash = DeploymentHash::new("_indexer").unwrap();
    static ref NETWORK: String = String::from("");
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
        println!("{:?}", &config);
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
         */
        /*
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
        /*
        let conn = connection.get_with_timeout_warning(&logger)?;
        let site = Connection::new(conn)
            .allocate_site(PRIMARY_SHARD.clone(), &DEPLOYMENT_HASH, NETWORK.clone())
            .unwrap();
         */
        /*
        let connection = Self::main_pool(
            &logger,
            indexer,
            PRIMARY_SHARD.as_str(),
            &shard_config,
            registry,
        );
         */
        match Self::create_relational_schema(schema_path, &connection) {
            Ok(layout) => {
                Self::create_relationships(&layout, &connection);
                Ok(PostgresIndexStore {
                    connection,
                    layout,
                    logger,
                })
            }
            Err(e) => Err(e.into()),
        }
    }
    /// Create a connection pool for the main database of hte primary shard
    /// without connecting to all the other configured databases
    pub fn main_pool(
        logger: &Logger,
        indexer: &str,
        name: &str,
        shard: &ShardConfig,
        registry: Arc<dyn MetricsRegistry>,
    ) -> ConnectionPool {
        let indexer_name = indexer.to_string();
        //let node = NodeId::new(indexer_name).unwrap();
        let node = NodeId::new("_indexer").unwrap();
        /*
        let pool_size = shard.pool_size.size_for(&node, name).expect(&format!(
            "we can determine the pool size for store {}",
            name
        ));
        let fdw_pool_size = shard.fdw_pool_size.size_for(&node, name).expect(&format!(
            "we can determine the fdw pool size for store {}",
            name
        ));
        */
        let pool_size = 20;
        let fdw_pool_size = 20;
        info!(
            logger,
            "Connecting to Postgres";
            "url" => SafeDisplay(shard.connection.as_str()),
            "conn_pool_size" => pool_size,
            "weight" => shard.weight
        );
        ConnectionPool::create(
            name,
            "main",
            shard.connection.to_owned(),
            pool_size,
            Some(fdw_pool_size),
            &logger,
            registry.cheap_clone(),
        )
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

        //let query = format!("create schema {}", NAMESPACE.as_str());
        //conn.batch_execute(&*query).unwrap();
        //Layout::new(site, &schema, catalog, false)
        //Need execute command CREATE EXTENSION btree_gist; on db
        let logger = Logger::root(slog::Discard, o!());
        let conn = connection.get_with_timeout_warning(&logger).unwrap();
        /*
        let site = Connection::new(&conn)
            .allocate_site(PRIMARY_SHARD.clone(), &DEPLOYMENT_HASH, NETWORK.clone())
            .unwrap();
        */

        let site = make_dummy_site(
            deployment_hash.cheap_clone(),
            NAMESPACE.clone(),
            String::from(""),
        );

        //let exists = deployment::exists(&conn, &site)?;
        let exists = true;
        let arc_site = Arc::new(site);
        let catalog = Catalog::make_empty(arc_site.clone()).unwrap();
        match exists {
            true => Layout::new(arc_site, &schema, catalog, false),
            false => Layout::create_relational_schema(
                connection.get_with_timeout_warning(&logger)?.deref(),
                arc_site,
                &schema,
            ),
        }
    }

    pub fn create_relationships(
        layout: &Layout,
        connection: &ConnectionPool,
    ) -> Result<(), anyhow::Error> {
        match layout.gen_relationship() {
            Ok(sql) => {
                println!("{:?}", sql.join(";\n"));
                //let conn = connection.get_with_timeout_warning(&logger).unwrap();
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
    /*
    pub fn load_layout_fromdb(logger: &Logger, connection: &ConnectionPool) -> Layout {
        let query = r#"
            SELECT
                pgc.conname as constraint_name,
                kcu.table_name as table_name,
                CASE WHEN (pgc.contype = 'f') THEN kcu.COLUMN_NAME ELSE ccu.COLUMN_NAME END as column_name,
                CASE WHEN (pgc.contype = 'f') THEN ccu.TABLE_NAME ELSE (null) END as reference_table,
                CASE WHEN (pgc.contype = 'f') THEN ccu.COLUMN_NAME ELSE (null) END as reference_col
            FROM
                pg_constraint AS pgc
                JOIN pg_namespace nsp ON nsp.oid = pgc.connamespace
                JOIN pg_class cls ON pgc.conrelid = cls.oid
                JOIN information_schema.key_column_usage kcu ON kcu.constraint_name = pgc.conname
                LEFT JOIN information_schema.constraint_column_usage ccu ON pgc.conname = ccu.CONSTRAINT_NAME
                AND nsp.nspname = ccu.CONSTRAINT_SCHEMA
            WHERE ccu.table_schema = ? AND pgc.contype = 'f'
        "#;
        let conn = connection.get_with_timeout_warning(&logger).unwrap();
        /*
        let tables = sql_query(query)
            .bind::<Text, _>("public")
            .get_results::<Row>(&conn);
        println!("Result: {:?}", tables);
         */
        //conn.deref_mut().deref_mut().query();
        //let result = conn.batch_execute(query);
        Layout {
            tables: Default::default(),
        }
    }
    */
}
fn named_type(field_type: &q::Type) -> &str {
    match field_type {
        q::Type::NamedType(name) => name.as_str(),
        q::Type::ListType(child) => named_type(child),
        q::Type::NonNullType(child) => named_type(child),
    }
}
/*
pub trait SubgraphStoreExt {
    fn get_site(&self) -> Site;
}

impl SubgraphStoreExt for SubgraphStoreInner {
    fn get_site(&self) -> Site {
        self.site(&DeploymentHash);
    }
}
*/
pub trait RelationshipGenerator {
    fn gen_relationship(&self) -> Result<Vec<String>, anyhow::Error>;
}
impl RelationshipGenerator for Table {
    fn gen_relationship(&self) -> Result<Vec<String>, anyhow::Error> {
        let sql = self.columns.iter().filter(|&column| column.is_reference() && !column.is_list()).map(|column|{
                format!(r#"CONSTRAINT fk_{column_name} FOREIGN KEY("{column_name}") REFERENCES "{reference}"({reference_id})"#,
                                         reference = named_type(&column.field_type).to_snake_case(),
                                         reference_id = &PRIMARY_KEY_COLUMN.to_owned(),
                                         column_name = column.name)

        }).collect::<Vec<String>>();
        Ok(sql)
    }
}

impl RelationshipGenerator for Layout {
    fn gen_relationship(&self) -> Result<Vec<String>, anyhow::Error> {
        let mut sqls = Vec::default();
        self.tables.iter().for_each(|(key, table)| {
            if let Ok(mut tbl_rels) = table.gen_relationship() {
                sqls.extend(tbl_rels);
            }
        });
        Ok(sqls)
    }
}
