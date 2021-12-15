use crate::store::{CacheableStore, IndexerStore};
use log::error;
use massbit::prelude::{anyhow, CheapClone, DeploymentHash, Logger, Schema, StoreError};
use massbit_common::prelude::diesel::connection::SimpleConnection;
use massbit_common::prelude::diesel::{
    r2d2::{self, ConnectionManager},
    sql_query, PgConnection, RunQueryDsl,
};
use massbit_common::prelude::reqwest::Client;
use massbit_common::prelude::tokio_compat_02::FutureExt;
use massbit_common::prelude::{serde_json, slog};
use massbit_solana_sdk::store::IndexStore;
use massbit_store_postgres::command_support::catalog::Site;
use massbit_store_postgres::relational::Catalog;
use massbit_store_postgres::relational::Layout;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

pub struct StoreBuilder {}
impl StoreBuilder {
    pub fn create_store<P: AsRef<Path>>(
        connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
        db_schema: &str,
        network: String,
        indexer_hash: String,
        schema_path: P,
        deployment_hash: DeploymentHash,
    ) -> Result<impl IndexStore, anyhow::Error> {
        let conn = connection_pool.get()?;
        let conn = conn.deref();
        match sql_query(format!("create schema if not exists {}", db_schema)).execute(conn) {
            Ok(_) => {
                match Self::create_relational_layout(
                    conn,
                    schema_path,
                    db_schema,
                    deployment_hash.cheap_clone(),
                    network.as_str(),
                ) {
                    Ok(layout) => {
                        let sql = layout.as_ddl().map_err(|_| {
                            StoreError::Unknown(anyhow!("failed to generate DDL for layout"))
                        })?;
                        match conn.batch_execute(&sql) {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("{:?}", e);
                            }
                        }
                        if let Err(err) = Self::create_hasura_relations(&layout) {
                            error!("{:?}", &err);
                        }
                        let logger = Logger::root(slog::Discard, slog::o!());
                        let store = Arc::new(IndexerStore {
                            connection_pool,
                            layout,
                            logger,
                        });
                        // let writable_store: Arc<dyn WritableStore> =
                        //     store.clone().to_writable_store();
                        Ok(CacheableStore::new(store, indexer_hash))
                    }
                    Err(e) => Err(e.into()),
                }
            }
            Err(err) => Err(err.into()),
        }
    }
    pub fn create_relational_layout<P: AsRef<Path>>(
        conn: &PgConnection,
        path: P,
        schema_name: &str,
        deployment_hash: DeploymentHash,
        network: &str,
    ) -> Result<Layout, StoreError> {
        let mut schema_buffer = String::new();
        let mut file = File::open(path).expect("Unable to open file"); // Refactor: Config to download config file from IPFS instead of just reading from local
        file.read_to_string(&mut schema_buffer)
            .expect("Unable to read string");
        let schema = Schema::parse(schema_buffer.as_str(), deployment_hash.cheap_clone()).unwrap();
        //let logger = Logger::root(slog::Discard, slog::o!());
        //Create simple site
        let site = Site::new(&deployment_hash, schema_name, network);
        let arc_site = Arc::new(site);
        let catalog = Catalog::new(conn, arc_site.clone()).unwrap();
        Layout::new(arc_site, &schema, catalog)
    }
}
