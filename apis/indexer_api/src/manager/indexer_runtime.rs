use crate::orm::models::Indexer;
use crate::orm::schema::indexers::dsl as idx;
use crate::store::StoreBuilder;
use crate::{CHAIN_READER_URL, COMPONENT_NAME, GET_BLOCK_TIMEOUT_SEC, GET_STREAM_TIMEOUT_SEC};
use chain_solana::data_source::{DataSource, DataSourceTemplate};
use chain_solana::manifest::ManifestResolve;
use chain_solana::SolanaIndexerManifest;
use libloading::Library;
use massbit::blockchain::{Blockchain, TriggerFilter};
use massbit::components::link_resolver::LinkResolver as _;
use massbit::components::store::{DeploymentId, DeploymentLocator};
use massbit::data::indexer::MAX_SPEC_VERSION;
use massbit::ipfs_client::IpfsClient;
use massbit::ipfs_link_resolver::LinkResolver;
use massbit::prelude::anyhow::Context;
use massbit::prelude::{Arc, LoggerFactory, Stream};
use massbit::prelude::{DeploymentHash, Logger};
use massbit_common::prelude::diesel::{
    r2d2::{self, ConnectionManager},
    ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use massbit_common::prelude::r2d2::PooledConnection;
use massbit_common::prelude::tokio::time::{sleep, timeout, Duration};
use massbit_common::prelude::{anyhow, serde_json};
use massbit_grpc::firehose::bstream::stream_client::StreamClient;
use massbit_grpc::firehose::bstream::{BlockRequest, ChainType};
use massbit_solana_sdk::plugin::handler::SolanaHandler;
use massbit_solana_sdk::plugin::proxy::SolanaHandlerProxy;
use massbit_solana_sdk::plugin::{
    AdapterDeclaration, BlockResponse, MessageHandler, PluginRegistrar,
};
use massbit_solana_sdk::store::IndexStore;
use massbit_solana_sdk::types::SolanaFilter;
use std::collections::HashMap;
use std::env::temp_dir;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use tonic::transport::Channel;
use tonic::{Request, Streaming};
use tower::timeout::Timeout;
use uuid::Uuid;

pub struct AdapterHandler {
    pub lib: Arc<Library>,
    pub handler_proxies: Option<SolanaHandlerProxy>,
}
impl AdapterHandler {
    fn new(lib: Arc<Library>) -> AdapterHandler {
        AdapterHandler {
            lib,
            handler_proxies: None,
        }
    }
}
impl PluginRegistrar for AdapterHandler {
    fn register_solana_handler(&mut self, handler: Box<dyn SolanaHandler + Send + Sync>) {
        self.handler_proxies = Some(SolanaHandlerProxy::new(handler));
    }
}

pub struct IndexerRuntime {
    pub indexer: Indexer,
    pub manifest: SolanaIndexerManifest,
    pub schema_path: Option<PathBuf>,
    pub mapping_path: Option<PathBuf>,
    pub adapter_handler: Option<AdapterHandler>,
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
}
/// Static methods
impl IndexerRuntime {
    pub async fn new(
        indexer: Indexer,
        ipfs_client: Arc<IpfsClient>,
        connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
        logger: Logger,
    ) -> Option<Self> {
        let link_resolver = LinkResolver::from(ipfs_client.clone());
        //let deployment_hash = DeploymentHash::new("_indexer").unwrap();
        let mapping_path = Self::get_ipfs_file(ipfs_client.clone(), &indexer.mapping, "so").await;
        let schema_path =
            Self::get_ipfs_file(ipfs_client.clone(), &indexer.graphql, "graphql").await;
        let opt_manifest = match ipfs_client.cat_all(&indexer.manifest, None).await {
            Ok(content) => {
                Self::parse_manifest(&indexer.hash, &content.to_vec(), link_resolver, &logger)
                    .await
                    .ok()
            }
            Err(err) => None,
        };
        let verified = if let Some(manifest) = opt_manifest.as_ref() {
            Self::verify_manifest(manifest)
        } else {
            false
        };
        //get schema and mapping content from ipfs to temporary dir
        if verified && mapping_path.is_some() && schema_path.is_some() {
            let runtime = IndexerRuntime {
                indexer,
                manifest: opt_manifest.unwrap(),
                mapping_path,
                schema_path,
                adapter_handler: None,
                connection_pool,
            };
            return Some(runtime);
        } else {
            log::error!("Manifest is invalid!");
        }
        None
    }
    pub async fn parse_manifest(
        indexer_hash: &String,
        manifest: &Vec<u8>,
        link_resolver: LinkResolver,
        logger: &Logger,
    ) -> Result<SolanaIndexerManifest, anyhow::Error> {
        let raw_value: serde_yaml::Value = serde_yaml::from_slice(&manifest).unwrap();
        let raw_map = match &raw_value {
            serde_yaml::Value::Mapping(m) => m,
            _ => panic!("Wrong type raw_manifest"),
        };
        //let deployment_hash = DeploymentHash::new(indexer_hash.clone()).unwrap();
        //Get raw manifest
        SolanaIndexerManifest::resolve_from_raw(
            logger,
            indexer_hash.clone(),
            raw_map.clone(),
            // Allow for infinite retries for indexer definition files.
            &link_resolver.with_retries(),
            MAX_SPEC_VERSION.clone(),
        )
        .await
        .context("Failed to resolve manifest from upload data")
    }
    async fn get_ipfs_file(
        ipfs_client: Arc<IpfsClient>,
        hash: &String,
        file_ext: &str,
    ) -> Option<PathBuf> {
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
    }
    pub fn verify_manifest(manifest: &SolanaIndexerManifest) -> bool {
        /// Manifest must contain single datasource
        if manifest.data_sources.len() != 1 {
            return false;
        }
        true
    }
}
impl IndexerRuntime {
    pub fn get_connection(
        &self,
    ) -> Result<
        PooledConnection<ConnectionManager<PgConnection>>,
        massbit_common::prelude::r2d2::Error,
    > {
        self.connection_pool.get()
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let templates = self
            .manifest
            .templates
            .iter()
            .map(|tpl| tpl.clone())
            .collect::<Vec<DataSourceTemplate>>();

        let _arc_templates = Arc::new(templates);
        let data_source = self.manifest.data_sources.get(0).unwrap();
        let mut network = String::default();
        if let Some(val) = &data_source.network {
            network = val.clone()
        }
        log::info!(
            "{} Init Streamout client for chain {} from block {:?} using language {}",
            &*COMPONENT_NAME,
            &data_source.kind,
            data_source.source.start_block,
            &data_source.mapping.language
        );
        //Create indexer database store
        let connection_pool = self.connection_pool.clone();
        let db_schema = self.indexer.namespace.as_str();
        let schema_path = self.schema_path.clone().unwrap();
        let deployment_hash = DeploymentHash::new("_indexer").unwrap();
        if let Ok(mut store) = StoreBuilder::create_store(
            connection_pool,
            db_schema,
            network,
            self.indexer.hash.clone(),
            schema_path,
            deployment_hash,
        ) {
            unsafe {
                match self.load_mapping_library(&store).await {
                    Ok(_) => {
                        log::info!("{} Load library successfully", &*COMPONENT_NAME);
                    }
                    Err(err) => {
                        log::error!("Load library with error {:?}", &err);
                        return Err(err);
                    }
                };
            }
            self.start_mapping(&mut store).await;
        }
        Ok(())
    }
    /// Load a plugin library
    /// A plugin library **must** be implemented using the
    /// [`model::adapter_declaration!()`] macro. Trying manually implement
    /// a plugin without going through that macro will result in undefined
    /// behaviour.use massbit::ipfs_link_resolver::LinkResolver;
    pub async unsafe fn load_mapping_library(
        &mut self,
        store: &dyn IndexStore,
    ) -> Result<(), Box<dyn Error>> {
        let library_path = self.mapping_path.as_ref().unwrap().as_os_str();
        let lib = Arc::new(Library::new(library_path)?);
        // inject store to plugin
        lib.get::<*mut Option<&dyn IndexStore>>(b"STORE\0")?
            .write(Some(store));
        let adapter_decl = lib
            .get::<*mut AdapterDeclaration>(b"adapter_declaration\0")?
            .read();
        let mut registrar = AdapterHandler::new(lib);
        (adapter_decl.register)(&mut registrar);
        self.adapter_handler = Some(registrar);
        Ok(())
    }
    async fn start_mapping(&mut self, store: &mut dyn IndexStore) -> Result<(), Box<dyn Error>> {
        if let Some(adapter) = &self.adapter_handler {
            if let Some(proxy) = &adapter.handler_proxies {
                let data_source = self.manifest.data_sources.get(0).unwrap();
                let mut opt_stream: Option<Streaming<BlockResponse>> = None;

                loop {
                    match opt_stream {
                        None => {
                            opt_stream = self.try_create_block_stream(data_source).await;
                            if opt_stream.is_none() {
                                //Sleep for a while and reconnect
                                sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC)).await;
                            }
                        }
                        Some(ref mut stream) => {
                            let response = timeout(
                                Duration::from_secs(GET_BLOCK_TIMEOUT_SEC),
                                stream.message(),
                            )
                            .await;
                            match response {
                                Ok(Ok(res)) => {
                                    if let Some(mut data) = res {
                                        match proxy.handle_block_mapping(&mut data, store) {
                                            Err(err) => {
                                                log::error!(
                                                    "{} Error while handle received message",
                                                    err
                                                );
                                            }
                                            Ok(block_slot) => {
                                                self.indexer.got_block = block_slot as i64;
                                                //Store got_block to db
                                                if let Ok(conn) = self.get_connection() {
                                                    if let Err(err) =
                                                        diesel::update(idx::indexers.filter(
                                                            idx::hash.eq(&self.indexer.hash),
                                                        ))
                                                        .set(idx::got_block.eq(block_slot as i64))
                                                        .execute(conn.deref())
                                                    {
                                                        log::error!("{:?}", &err);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    log::info!(
                                "Error while get message from reader stream {:?}. Recreate stream",
                                &response
                            );
                                    opt_stream = None;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    async fn try_create_block_stream(
        &self,
        data_source: &DataSource,
    ) -> Option<Streaming<BlockResponse>> {
        //Todo: if remove this line, debug will be broken
        // let _filter =
        //     <chain_solana::Chain as Blockchain>::TriggerFilter::from_data_sources(vec![].iter());
        let addresses = match &data_source.source.address {
            Some(addr) => vec![addr.as_str()],
            None => vec![],
        };
        let filter = SolanaFilter::new(addresses);
        let encoded_filter = serde_json::to_vec(&filter).unwrap();
        let start_block_number = if self.indexer.got_block >= 0 {
            Some(self.indexer.got_block.clone() as u64 + 1)
        } else {
            None
        };
        log::info!(
            "Indexer {:?} get new stream from block {:?}.",
            &self.indexer.name,
            start_block_number
        );
        let chain_type = match data_source.kind.split('/').next().unwrap() {
            "ethereum" => ChainType::Ethereum,
            _ => ChainType::Solana, // If not provided, assume it's Solana network
        };
        let transaction_request = BlockRequest {
            start_block_number,
            chain_type: chain_type as i32,
            network: data_source.network.clone().unwrap_or(Default::default()),
            filter: encoded_filter,
        };
        if let Ok(channel) = Channel::from_static(CHAIN_READER_URL.as_str())
            .connect()
            .await
        {
            let timeout_channel = Timeout::new(channel, Duration::from_secs(GET_BLOCK_TIMEOUT_SEC));
            let mut client = StreamClient::new(timeout_channel);
            match client
                .blocks(Request::new(transaction_request.clone()))
                .await
            {
                Ok(res) => Some(res.into_inner()),
                Err(err) => {
                    log::error!("Create new stream with error {:?}", &err);
                    None
                }
            }
        } else {
            log::error!(
                "Cannot connect to chain reader at address {:?}",
                CHAIN_READER_URL.as_str()
            );
            None
        }
    }

    // async fn handle_rust_mapping<P: AsRef<Path>>(
    //     &mut self,
    //     indexer_hash: &String,
    //     db_schema: &String,
    //     data_source: &DataSource,
    //     init_block: u64,
    //     mapping_path: P,
    //     schema_path: P,
    //     client: &mut StreamClient<Timeout<Channel>>,
    // ) -> Result<(), anyhow::Error> {
    //     let store = StoreBuilder::create_store(db_schema.as_str(), &schema_path).unwrap();
    //     let mut indexer_state = IndexerState::new(Arc::new(store));
    //
    //     //Use unsafe to inject a store pointer into user's lib
    //     unsafe {
    //         match self
    //             .load(
    //                 indexer_hash,
    //                 mapping_path.as_ref().as_os_str(),
    //                 &indexer_state,
    //             )
    //             .await
    //         {
    //             Ok(_) => log::info!("{} Load library successfully", &*COMPONENT_NAME),
    //             Err(err) => log::error!("Load library with error {:?}", err),
    //         }
    //     }
    //     log::info!("{} Start mapping using rust", &*COMPONENT_NAME);
    //     let adapter_name = data_source
    //         .kind
    //         .split("/")
    //         .collect::<Vec<&str>>()
    //         .get(0)
    //         .unwrap()
    //         .to_string();
    //     if let Some(adapter_handler) = self.map_handlers.get_mut(indexer_hash.as_str()) {
    //         if let Some(handler_proxy) = adapter_handler.handler_proxies.get(&adapter_name) {
    //             let mut start_block = init_block;
    //             let chain_type = get_chain_type(data_source);
    //             let mut opt_stream: Option<Streaming<BlockResponse>> = None;
    //             log::info!(
    //                 "Rust mapping get new stream for chain {:?} from block {}.",
    //                 &chain_type,
    //                 start_block
    //             );
    //             loop {
    //                 match opt_stream {
    //                     None => {
    //                         opt_stream =
    //                             try_create_transaction_stream(client, start_block, data_source)
    //                                 .await;
    //                         if opt_stream.is_none() {
    //                             //Sleep for a while and reconnect
    //                             sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC)).await;
    //                         }
    //                     }
    //                     Some(ref mut stream) => {
    //                         let response = timeout(
    //                             Duration::from_secs(GET_BLOCK_TIMEOUT_SEC),
    //                             stream.message(),
    //                         )
    //                         .await;
    //                         match response {
    //                             Ok(Ok(res)) => {
    //                                 if let Some(mut data) = res {
    //                                     match handler_proxy
    //                                         .handle_block_mapping(&mut data, &mut indexer_state)
    //                                     {
    //                                         Err(err) => {
    //                                             log::error!(
    //                                                 "{} Error while handle received message",
    //                                                 err
    //                                             );
    //                                             //start_block = data.block_number;
    //                                         }
    //                                         Ok(_) => {
    //                                             // start_block = data.block_number + 1;
    //                                             // //Store got_block to db
    //                                             // IndexerStore::store_got_block(
    //                                             //     indexer_hash,
    //                                             //     data.block_number as i64,
    //                                             // );
    //                                         }
    //                                     }
    //                                 }
    //                             }
    //                             _ => {
    //                                 log::info!("Error while get message from reader stream {:?}. Recreate stream", &response);
    //                                 opt_stream = None;
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //         } else {
    //             log::debug!(
    //                 "{} Cannot find proxy for adapter {}",
    //                 *COMPONENT_NAME,
    //                 adapter_name
    //             );
    //         }
    //     } else {
    //         log::debug!(
    //             "{} Cannot find adapter handler for indexer {}",
    //             &*COMPONENT_NAME,
    //             &indexer_hash
    //         );
    //     }
    //     Ok(())
    // }
}
