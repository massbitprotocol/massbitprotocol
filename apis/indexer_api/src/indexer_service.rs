use crate::orm::models::Indexer;
use crate::orm::schema::indexers;
use adapter::core::AdapterManager;
use chain_solana::manifest::ManifestResolve;
use chain_solana::SolanaIndexerManifest;
use diesel::sql_types::BigInt;
use log::{debug, info};
use massbit::components::link_resolver::LinkResolver as _;
use massbit::components::store::{DeploymentId, DeploymentLocator};
use massbit::data::indexer::DeploymentHash;
use massbit::data::indexer::MAX_SPEC_VERSION;
use massbit::ipfs_client::IpfsClient;
use massbit::ipfs_link_resolver::LinkResolver;
use massbit::prelude::anyhow::Context;
use massbit::prelude::prost::bytes::BufMut;
use massbit::prelude::{anyhow, CheapClone, LoggerFactory, TryStreamExt};
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{r2d2, PgConnection, RunQueryDsl};
use massbit_common::prelude::r2d2::PooledConnection;
use std::env::temp_dir;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use warp::{
    multipart::{FormData, Part},
    Rejection, Reply,
};

pub struct IndexerService {
    pub ipfs_clients: Vec<IpfsClient>,
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
    pub logger_factory: LoggerFactory,
}

impl IndexerService {
    pub fn get_connection(
        &self,
    ) -> Result<
        PooledConnection<ConnectionManager<PgConnection>>,
        massbit_common::prelude::r2d2::Error,
    > {
        self.connection_pool.get()
    }
    pub async fn deploy_indexer(&self, form: FormData) -> Result<impl Reply, Rejection> {
        log::info!("Deploy new indexer");
        //let mut store_path: Option<String> = None;
        let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
            eprintln!("form error: {}", e);
            warp::reject::reject()
        })?;
        let ipfs_client = self.ipfs_clients.get(0);
        let mut indexer = Indexer::new();
        let mut manifest: Option<SolanaIndexerManifest> = None;
        for p in parts {
            log::info!("Receive file: {}/{}", &p.name(), p.filename().unwrap());
            let p_name = format!("{}", &p.name());
            match p_name.as_str() {
                name @ "mapping" | name @ "schema" | name @ "manifest" => {
                    let value = p
                        .stream()
                        .try_fold(Vec::new(), |mut vec, data| {
                            vec.put(data);
                            async move { Ok(vec) }
                        })
                        .await
                        .map_err(|e| {
                            eprintln!("reading file error: {}", e);
                            warp::reject::reject()
                        })?;
                    if name == "manifest" {
                        manifest = self.parse_manifest(&indexer.hash, &value).await.ok();
                    }
                    if let Some(ipfs) = ipfs_client {
                        match ipfs.add(value).await {
                            Ok(response) => match name {
                                "mapping" => indexer.mapping = response.hash.clone(),
                                "schema" => indexer.graphql = response.hash.clone(),
                                "manifest" => indexer.manifest = response.hash.clone(),
                                &_ => {}
                            },
                            Err(err) => {
                                log::error!("{:?}", &err);
                            }
                        }
                    } else {
                        log::warn!("Ipfs client not configured");
                    }
                }
                _ => {}
            }
        }
        if let Some(manifest) = &manifest {
            if let Some(datasource) = manifest.data_sources.get(0) {
                indexer.address = datasource.source.address.clone();
                indexer.start_block = datasource.source.start_block as i64;
                indexer.network = datasource.network.clone();
                indexer.name = datasource.name.clone();
                indexer.status = Some(String::from("Deploying"));
            }
            if let Ok(conn) = self.get_connection() {
                indexer.v_id = self.get_next_sequence(&conn, "indexers", "v_id");
                indexer.namespace = format!("sgd{}", indexer.v_id);
                //let indexer = indexer.clone();
                let inserted_value = diesel::insert_into(indexers::table)
                    .values(&indexer)
                    .get_result::<Indexer>(&conn)
                    .expect("Error while create new indexer");
            }
            if let Err(err) = self.init_indexer(&indexer, manifest).await {
                log::error!("{:?}", &err);
            }
        }
        Ok("success")
    }
    pub async fn parse_manifest(
        &self,
        hash: &String,
        manifest: &Vec<u8>,
    ) -> Result<SolanaIndexerManifest, anyhow::Error> {
        let raw_value: serde_yaml::Value = serde_yaml::from_slice(&manifest).unwrap();
        let raw_map = match &raw_value {
            serde_yaml::Value::Mapping(m) => m,
            _ => panic!("Wrong type raw_manifest"),
        };
        let deployment_hash = DeploymentHash::new(hash.clone()).unwrap();
        let link_resolver = LinkResolver::from(self.ipfs_clients.clone());
        let logger = self.logger_factory.indexer_logger(&DeploymentLocator::new(
            DeploymentId(0),
            deployment_hash.clone(),
        ));
        //Get raw manifest
        SolanaIndexerManifest::resolve_from_raw(
            &logger,
            deployment_hash.cheap_clone(),
            raw_map.clone(),
            // Allow for infinite retries for indexer definition files.
            &link_resolver.with_retries(),
            MAX_SPEC_VERSION.clone(),
        )
        .await
        .context("Failed to resolve manifest from upload data")
    }
    fn get_next_sequence(
        &self,
        conn: &PooledConnection<ConnectionManager<PgConnection>>,
        table: &str,
        column: &str,
    ) -> i64 {
        let sql = format!(
            r#"SELECT nextval(pg_get_serial_sequence('{}', '{}')) as value;"#,
            table, column
        );
        #[derive(Debug, Default, QueryableByName)]
        struct SequenceNumber {
            #[sql_type = "BigInt"]
            pub value: i64,
        }
        let next_seq = diesel::sql_query(sql.clone()).get_result::<SequenceNumber>(conn);
        log::info!("{}, {:?}", &sql, &next_seq);
        next_seq.unwrap_or_default().value
    }
    pub async fn init_indexer(
        &self,
        indexer: &Indexer,
        manifest: &SolanaIndexerManifest,
    ) -> Result<(), anyhow::Error> {
        log::info!("Init indexer {:?} {:?}", &indexer.hash, &indexer.name);
        //get schema and mapping content from ipfs to temporary dir
        let mapping_path = self.get_ipfs_file(&indexer.mapping, "so").await;
        let schema_path = self.get_ipfs_file(&indexer.graphql, "graphql").await;
        if mapping_path.is_some() && schema_path.is_some() {
            let mut adapter = AdapterManager::new();
            adapter
                .init(
                    &indexer.hash,
                    &indexer.namespace,
                    mapping_path.as_ref().unwrap(),
                    schema_path.as_ref().unwrap(),
                    manifest,
                )
                .await?;
        }

        Ok(())
    }
    async fn get_ipfs_file(&self, hash: &String, file_ext: &str) -> Option<PathBuf> {
        if let Some(ipfs_client) = self.ipfs_clients.get(0) {
            ipfs_client
                .cat_all(hash, None)
                .await
                .ok()
                .and_then(|content| {
                    let mut dir = temp_dir();
                    let file_name = format!("{}.{}", Uuid::new_v4(), file_ext);
                    //println!("{}", file_name);
                    dir.push(file_name);
                    fs::write(&dir, content.to_vec());
                    //let file = File::create(dir)?;
                    log::info!(
                        "Download content of file {} into {}",
                        hash,
                        dir.to_str().unwrap()
                    );
                    Some(dir)
                })
        } else {
            None
        }
    }
}
