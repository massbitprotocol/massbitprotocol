use crate::orm::models::Indexer;
use crate::orm::schema::indexers::dsl as idx;
use crate::store::StoreBuilder;
use crate::{CHAIN_READER_URL, COMPONENT_NAME, GET_BLOCK_TIMEOUT_SEC, GET_STREAM_TIMEOUT_SEC};
use chain_solana::adapter::SolanaNetworkAdapter;
use chain_solana::data_source::{DataSource, DataSourceTemplate};
use chain_solana::manifest::ManifestResolve;
use chain_solana::types::{Pubkey, SolanaFilter};
use chain_solana::SolanaIndexerManifest;
use libloading::Library;
use log::info;
use massbit::components::link_resolver::LinkResolver as _;
use massbit::data::indexer::MAX_SPEC_VERSION;
use massbit::ipfs_client::IpfsClient;
use massbit::ipfs_link_resolver::LinkResolver;
use massbit::prelude::anyhow::Context;
use massbit::prelude::Arc;
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
use massbit_solana_sdk::plugin::{AdapterDeclaration, BlockResponse, PluginRegistrar};
use massbit_solana_sdk::store::IndexStore;
use massbit_solana_sdk::types::{ExtBlock, SolanaBlock};
use solana_sdk::signature::Signature;
use std::env::temp_dir;
use std::error::Error;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::{fs, thread};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tonic::transport::Channel;
use tonic::{Request, Streaming};
use tower::timeout::Timeout;
use uuid::Uuid;

pub struct IndexerHandler {
    pub lib: Arc<Library>,
    pub handler_proxies: Option<Arc<SolanaHandlerProxy>>,
}
impl IndexerHandler {
    fn new(lib: Arc<Library>) -> IndexerHandler {
        IndexerHandler {
            lib,
            handler_proxies: None,
        }
    }
}
impl PluginRegistrar for IndexerHandler {
    fn register_solana_handler(&mut self, handler: Box<dyn SolanaHandler + Send + Sync>) {
        self.handler_proxies = Some(Arc::new(SolanaHandlerProxy::new(handler)));
    }
}

pub struct IndexerRuntime {
    pub indexer: Indexer,
    pub manifest: SolanaIndexerManifest,
    pub schema_path: Option<PathBuf>,
    pub mapping_path: Option<PathBuf>,
    pub indexer_handler: Option<IndexerHandler>,
    pub network_adapter: Arc<SolanaNetworkAdapter>,
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
        let verified = opt_manifest
            .as_ref()
            .and_then(|manifest| Some(Self::verify_manifest(manifest)))
            .unwrap_or(false);
        //get schema and mapping content from ipfs to temporary dir
        if verified && mapping_path.is_some() && schema_path.is_some() {
            let adapter = SolanaNetworkAdapter::from(indexer.network.clone().unwrap_or_default());
            let manifest = opt_manifest.unwrap();
            let data_source = manifest.data_sources.get(0).unwrap();
            let runtime = IndexerRuntime {
                indexer,
                manifest,
                mapping_path,
                schema_path,
                indexer_handler: None,
                network_adapter: Arc::new(adapter),
                connection_pool,
            };
            return Some(runtime);
        } else {
            log::error!("Manifest is invalid!");
        }
        None
    }
}
impl<'a> IndexerRuntime {
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
                match self.load_mapping_library(&mut store).await {
                    Ok(_) => {
                        log::info!("{} Load library successfully", &*COMPONENT_NAME);
                    }
                    Err(err) => {
                        log::error!("Load library with error {:?}", &err);
                        return Err(err);
                    }
                };
            }
            {
                // let store: Arc<Mutex<Box<&mut dyn IndexStore>>> =
                //     Arc::new(Mutex::new(Box::new(&mut store)));
                self.start_mapping().await;
            }
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
        store: &mut dyn IndexStore,
    ) -> Result<(), Box<dyn Error>> {
        let library_path = self.mapping_path.as_ref().unwrap().as_os_str();
        let lib = Arc::new(Library::new(library_path)?);
        // inject store to plugin
        lib.get::<*mut Option<&dyn IndexStore>>(b"STORE\0")?
            .write(Some(store));
        let adapter_decl = lib
            .get::<*mut AdapterDeclaration>(b"adapter_declaration\0")?
            .read();
        let mut registrar = IndexerHandler::new(lib);
        (adapter_decl.register)(&mut registrar);
        self.indexer_handler = Some(registrar);
        Ok(())
    }
    async fn start_mapping(
        &mut self,
        //store: Arc<Mutex<Box<&mut dyn IndexStore>>>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(adapter) = &self.indexer_handler {
            if let Some(proxy) = &adapter.handler_proxies {
                let data_source = self.manifest.data_sources.get(0).unwrap();
                let mut opt_stream: Option<Streaming<BlockResponse>> = None;
                let mut start_block = if self.indexer.got_block >= 0 {
                    Some(self.indexer.got_block.clone() as u64 + 1)
                } else {
                    None
                };
                let (history_block_tx, mut history_block_rx) =
                    mpsc::channel::<Vec<SolanaBlock>>(64);
                loop {
                    // Todo: decide how indexer handle if some error occurred during phase get history data.
                    // And sometime there are many gaps of blocks need to filled.
                    // For example: chain reader send blocks B_1, B_n, B_m, need to get [B2..B_n), (B_n, B_m)
                    //Process all history blocks before handle current blocks
                    while let Ok(blocks) = history_block_rx.try_recv() {
                        match proxy.handle_blocks(&blocks) {
                            Err(err) => {
                                log::error!("{:?} Error while handle history blocks", &err);
                            }
                            Ok(block_slot) => {
                                log::info!("Process to block: {:?}", block_slot);
                            }
                        }
                    }
                    match opt_stream {
                        None => {
                            opt_stream = self
                                .try_create_block_stream(data_source, start_block.clone())
                                .await;
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
                                        let blocks: Vec<SolanaBlock> =
                                            serde_json::from_slice(&mut data.payload).unwrap();
                                        //Get history block from first transaction in first block
                                        if let Some(block) = blocks.get(0) {
                                            if block.block.parent_slot
                                                > self.indexer.got_block as u64
                                            {
                                                let from_signature = block
                                                    .block
                                                    .transactions
                                                    .first()
                                                    .unwrap()
                                                    .transaction
                                                    .signatures
                                                    .first()
                                                    .and_then(|sig| Some(sig.clone()));
                                                self.load_history_data(
                                                    history_block_tx.clone(),
                                                    self.indexer.got_block as u64,
                                                    from_signature,
                                                )
                                                .await;
                                            }
                                        }

                                        match proxy.handle_blocks(&blocks) {
                                            Err(err) => {
                                                log::error!(
                                                    "{} Error while handle received message",
                                                    err
                                                );
                                            }
                                            Ok(block_slot) => {
                                                self.indexer.got_block = block_slot;
                                                start_block = Some(block_slot as u64 + 1);
                                                //Store got_block to db
                                                if let Ok(conn) = self.get_connection() {
                                                    if let Err(err) =
                                                        diesel::update(idx::indexers.filter(
                                                            idx::hash.eq(&self.indexer.hash),
                                                        ))
                                                        .set(idx::got_block.eq(block_slot))
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
}
