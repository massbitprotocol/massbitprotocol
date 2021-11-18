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
    pub logger_factory: LoggerFactory, // pub task_sender: Sender<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>,
                                       // pub task_receiver: Receiver<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>
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
                    // if store_path.is_none() {
                    //     store_path = Some(uuid::Uuid::new_v4().to_string());
                    // }
                    // let file_name = format!(
                    //     "{}/{}/{}",
                    //     INDEXER_UPLOAD_DIR.deref(),
                    //     store_path.as_ref().unwrap(),
                    //     p.filename().unwrap()
                    // );
                    // //create directory
                    // // Check and create parent directory
                    // let path = std::path::Path::new(&file_name);
                    // let prefix = path.parent().unwrap();
                    // std::fs::create_dir_all(prefix);
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
                    // tokio::fs::write(&file_name, value).await.map_err(|e| {
                    //     eprint!("error writing file: {}", e);
                    //     warp::reject::reject()
                    // })?;
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

        // if let Some(address) = raw_value["dataSources"][0]["source"]["address"].as_str() {
        //     indexer.address = Some(String::from(address));
        // }
        // match &raw_value["dataSources"][0]["source"]["start_block"] {
        //     serde_yaml::Value::Number(num) => {
        //         indexer.start_block = num.as_i64().unwrap_or_default();
        //     }
        //     _ => {}
        // }
        // if let Some(network) = raw_value["dataSources"][0]["kind"].as_str() {
        //     indexer.network = Some(String::from(network));
        // };
        // if let Some(name) = raw_value["dataSources"][0]["name"].as_str() {
        //     indexer.name = String::from(name);
        // };
        // indexer.status = Some(String::from("deployed"));
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

// async fn deploy_handler(params: DeployParams) -> JsonRpcResult<Value> {
//     println!("Params {:?}", &params);
//     let index_config = IndexConfigIpfsBuilder::default()
//         .config(&params.config)
//         .await
//         .mapping(&params.mapping)
//         .await
//         .schema(&params.schema)
//         .await
//         //.abi(params.abi)
//         //.await
//         .subgraph(&params.subgraph)
//         .await
//         .build();
//     // Set up logger
//     let logger = logger(false);
//     let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
//     let ipfs_clients: Vec<IpfsClient> = create_ipfs_clients(&ipfs_addresses).await;
//
//     // Convert the clients into a link resolver. Since we want to get past
//     // possible temporary DNS failures, make the resolver retry
//     let link_resolver = Arc::new(LinkResolver::from(ipfs_clients));
//     // Create a component and indexer logger factory
//     let logger_factory = LoggerFactory::new(logger.clone());
//     let deployment_hash = DeploymentHash::new(index_config.identifier.hash.clone())?;
//     let logger = logger_factory.indexer_logger(&DeploymentLocator::new(
//         DeploymentId(0),
//         deployment_hash.clone(),
//     ));
//     info!("Ipfs {:?}", &deployment_hash.to_ipfs_link());
//     // let raw: serde_yaml::Mapping = {
//     //     let file_bytes = link_resolver
//     //         .cat(&logger, &deployment_hash.to_ipfs_link())
//     //         .await
//     //         .map_err(|e| {
//     //             error!("{:?}", &e);
//     //             IndexerRegistrarError::ResolveError(IndexerManifestResolveError::ResolveError(e))
//     //         })?;
//     //
//     //     serde_yaml::from_slice(&file_bytes)
//     //         .map_err(|e| IndexerRegistrarError::ResolveError(e.into()))?
//     // };
//     // TODO: Maybe break this into two different struct (So and Wasm) so we don't have to use Option
//     // let mut manifest = IndexerManifest::<Chain>::resolve_from_raw(
//     //     &logger,
//     //     deployment_hash.cheap_clone(),
//     //     raw,
//     //     // Allow for infinite retries for indexer definition files.
//     //     &link_resolver.as_ref().clone().with_retries(),
//     //     MAX_SPEC_VERSION.clone(),
//     // )
//     // .await
//     // .context("Failed to resolve indexer from IPFS")?;
//     let manifest: Option<IndexerManifest<Chain>> = match &params.subgraph {
//         Some(v) => Some(
//             get_indexer_manifest(DeploymentHash::new(v)?, link_resolver)
//                 .await
//                 .unwrap(),
//         ),
//         None => {
//             println!(".SO mapping doesn't have parsed data source");
//             //vec![]
//             None
//         }
//     };
//     // Create tables for the new index and track them in hasura
//     //run_ddl_gen(&index_config).await;
//
//     // Create a new indexer so we can keep track of it's status
//     //IndexStore::insert_new_indexer(&index_config);
//     let config_value = read_config_file(&index_config.config);
//     let network = config_value["dataSources"][0]["kind"].as_str().unwrap();
//     let name = config_value["dataSources"][0]["name"].as_str().unwrap();
//     IndexerStore::create_indexer(
//         index_config.identifier.hash.clone(),
//         String::from(name),
//         String::from(network),
//         &params.subgraph,
//     );
//     //Start the adapter for the index
//     adapter_init(&index_config, &manifest).await?;
//
//     let res = Output::from(
//         Ok(serde_json::to_value("Deploy index success").expect("Unable to deploy new index")),
//         Id::Num(2),
//         None,
//     );
//     let success = Success {
//         jsonrpc: Some(Version::V2),
//         result: json!("Deploy index success"),
//         id: Id::Num(2),
//     };
//     Ok(json!(success))
// }
