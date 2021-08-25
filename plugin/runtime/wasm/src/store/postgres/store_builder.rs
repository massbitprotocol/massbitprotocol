//use super::relational::Layout;
use crate::mock::MockMetricsRegistry;
use crate::prelude::{Arc, Logger};
use graph_store_postgres::PRIMARY_SHARD;
/*
use crate::store::postgres::ConnectionPool;
 */
//use crate::store::postgres::ConnectionPool;
use crate::store::PostgresIndexStore;
use core::ops::{Deref, DerefMut};
use graph::cheap_clone::CheapClone;
use graph::components::metrics::MetricsRegistry;
use graph::data::schema::Schema;
use graph::log::logger;
use graph::prelude::{DeploymentHash, NodeId, StoreError};
use graph::util::security::SafeDisplay;
use graph_node::config::{PoolSize, Shard as ShardConfig};

use graph_store_postgres::layout_for_tests::make_dummy_site;
use graph_store_postgres::{
    command_support::{Catalog, Layout, Namespace},
    connection_pool::ConnectionPool,
};
use massbit_common::prelude::diesel::connection::SimpleConnection;
use massbit_common::prelude::diesel::sql_types::Text;
use massbit_common::prelude::diesel::{sql_query, RunQueryDsl};
use massbit_common::prelude::lazy_static::lazy_static;
use massbit_common::prelude::tokio_postgres::Row;
use slog::{info, o};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

lazy_static! {
    //All indexer write into main public schema
    static ref NAMESPACE: Namespace = Namespace::new("sgd0".to_string()).unwrap();
}
pub fn create_store<P: AsRef<Path>>(
    indexer: &str,
    connection_string: &str,
    schema_path: &P,
) -> PostgresIndexStore {
    let logger = logger(false);
    let config = ShardConfig {
        connection: connection_string.to_string(),
        weight: 0,
        pool_size: PoolSize::Fixed(20),
        fdw_pool_size: Default::default(),
        replicas: Default::default(),
    };
    let registry = Arc::new(MockMetricsRegistry::new());
    let connection = main_pool(&logger, indexer, PRIMARY_SHARD.as_str(), &config, registry);
    //let layout = load_layout(&logger, &connection);
    let layout = parse_layout(indexer, schema_path, &connection).unwrap();
    PostgresIndexStore {
        connection_string: connection_string.to_string(),
        connection,
        layout,
        logger,
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
pub fn parse_layout<P: AsRef<Path>>(
    indexer_hash: &str,
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
    let site = Arc::new(make_dummy_site(
        deployment_hash.cheap_clone(),
        NAMESPACE.clone(),
        String::from(""),
    ));
    let catalog = Catalog::make_empty(site.clone()).unwrap();
    //let query = format!("create schema {}", NAMESPACE.as_str());
    //conn.batch_execute(&*query).unwrap();
    Layout::new(site, &schema, catalog, false)
    /*
    let logger = Logger::root(slog::Discard, o!());
    Layout::create_relational_schema(
        connection.get_with_timeout_warning(&logger)?.deref(),
        site.clone(),
        &schema,
    )
     */
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
