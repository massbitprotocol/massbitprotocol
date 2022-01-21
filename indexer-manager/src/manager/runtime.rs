use crate::manager::buffer::IncomingBlocks;
use crate::store::StoreBuilder;
use crate::{CHAIN_READER_URL, COMPONENT_NAME, GET_BLOCK_TIMEOUT_SEC, GET_STREAM_TIMEOUT_SEC};
use crate::{INDEXER_PROCESS_THREAD_LIMIT, WAITING_FOR_INCOMING_BLOCK_MILLISECOND};
use chain_solana::adapter::{SolanaNetworkAdapter, SolanaNetworkAdapters};
use chain_solana::data_source::{DataSource, DataSourceTemplate};
use chain_solana::manifest::ManifestResolve;
use chain_solana::types::{Pubkey, SolanaFilter};
use chain_solana::SolanaIndexerManifest;
use diesel::{Connection, EqAll};
use indexer_orm::{models::Indexer, schema::*};
use libloading::{Library, Symbol};
use log::info;
use massbit::components::link_resolver::LinkResolver as _;
use massbit::data::indexer::MAX_SPEC_VERSION;
use massbit::ipfs_client::IpfsClient;
use massbit::ipfs_link_resolver::LinkResolver;
use massbit_common::prelude::anyhow::anyhow;
use massbit_common::prelude::bigdecimal::BigDecimal;
use massbit_common::prelude::diesel::{
    r2d2::{self, ConnectionManager},
    ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use massbit_common::prelude::r2d2::PooledConnection;
use massbit_common::prelude::tokio::time::{sleep, timeout, Duration, Instant};
use massbit_common::prelude::uuid::Uuid;
use massbit_common::prelude::{
    anyhow::{self, Context},
    serde_json,
    slog::Logger,
};
use massbit_data::indexer::DeploymentHash;
use massbit_grpc::firehose::bstream::stream_client::StreamClient;
use massbit_grpc::firehose::bstream::{BlockRequest, ChainType};
use massbit_solana_sdk::plugin::handler::SolanaHandler;
use massbit_solana_sdk::plugin::proxy::SolanaHandlerProxy;
use massbit_solana_sdk::plugin::{AdapterDeclaration, BlockResponse, PluginRegistrar};
use massbit_solana_sdk::smart_contract::{
    InstructionInterface, InstructionParser, InterfaceRegistrar, SmartContractProxy,
};
use massbit_solana_sdk::store::IndexStore;
use massbit_solana_sdk::types::{ExtBlock, SolanaBlock};
use massbit_solana_sdk::SmartContractRegistrar;
use solana_sdk::signature::Signature;
use solana_sdk::slot_history::Slot;
use std::env::temp_dir;
use std::error::Error;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::{fs, thread};
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Semaphore};
use tonic::transport::Channel;
use tonic::{Request, Streaming};
use tower::timeout::Timeout;

const DEFAULT_NETWORK: &str = "mainnet";
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
    pub unpack_instruction_path: Option<PathBuf>,
    pub indexer_handler: Option<IndexerHandler>,
    pub unpacking_handler: Option<SmartContractRegistrar>,
    block_buffer: Arc<IncomingBlocks>,
    got_block: Option<Slot>,
    pub network_adapters: Arc<Mutex<SolanaNetworkAdapters>>,
    pub connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
}
/// Static methods
impl IndexerRuntime {
    pub async fn new(
        indexer: Indexer,
        ipfs_client: Arc<IpfsClient>,
        connection_pool: Arc<r2d2::Pool<ConnectionManager<PgConnection>>>,
        block_buffer: Arc<IncomingBlocks>,
        logger: Logger,
    ) -> Option<Self> {
        let link_resolver = LinkResolver::from(ipfs_client.clone());
        let mapping_path = Self::get_ipfs_file(ipfs_client.clone(), &indexer.mapping, "so").await;
        let unpack_instruction_path =
            Self::get_ipfs_file(ipfs_client.clone(), &indexer.unpack_instruction, "so").await;
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
            let adapters = SolanaNetworkAdapters::new(
                indexer
                    .network
                    .as_ref()
                    .and_then(|val| Some(val.as_str()))
                    .unwrap_or(DEFAULT_NETWORK),
                None,
            );
            let manifest = opt_manifest.unwrap();
            let data_source = manifest.data_sources.get(0).unwrap();
            let runtime = IndexerRuntime {
                indexer,
                manifest,
                mapping_path,
                unpack_instruction_path,
                schema_path,
                indexer_handler: None,
                unpacking_handler: None,
                block_buffer,
                got_block: None,
                network_adapters: Arc::new(Mutex::new(adapters)),
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
        // Manifest must contain single datasource
        if manifest.data_sources.len() != 1 {
            return false;
        }
        true
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
        match StoreBuilder::create_store(
            connection_pool,
            db_schema,
            network,
            self.indexer.hash.clone(),
            schema_path,
            deployment_hash,
        ) {
            Ok(mut store) => {
                // Todo: reduce Unsafe code.
                unsafe {
                    if self.load_unpacking_library().is_ok() {
                        match self.load_mapping_library(&mut store).await {
                            Ok(_) => {
                                log::info!("{} Load library successfully", &*COMPONENT_NAME);
                            }
                            Err(err) => {
                                log::error!("Load library with error {:?}", &err);
                                return Err(err);
                            }
                        }
                    }
                }
                {
                    self.start_mapping().await;
                }
            }
            Err(err) => {
                println!("create_store error: {:?}", err);
            }
        };

        Ok(())
    }
    pub unsafe fn load_unpacking_library(&mut self) -> Result<(), Box<dyn Error>> {
        let unpack_instruction_path = self
            .unpack_instruction_path
            .as_ref()
            .unwrap()
            .clone()
            .into_os_string();
        let unpacking_lib = Arc::new(
            Library::new(unpack_instruction_path.as_os_str())
                .map_err(|err| anyhow!("{:?}", &err))?,
        );
        let sm_entrypoint = unpacking_lib
            .get::<*mut InstructionInterface>(b"entrypoint\0")
            .unwrap()
            .read();
        let mut sm_registrar = SmartContractRegistrar::new(unpacking_lib);
        (sm_entrypoint.register)(&mut sm_registrar);
        self.unpacking_handler = Some(sm_registrar);
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
        // let unpack_instruction_path = self
        //     .unpack_instruction_path
        //     .as_ref()
        //     .unwrap()
        //     .clone()
        //     .into_os_string();

        let lib = Arc::new(Library::new(library_path)?);
        // inject store to plugin
        lib.get::<*mut Option<&dyn IndexStore>>(b"STORE\0")?
            .write(Some(store));
        let proxy = self.unpacking_handler.as_ref().and_then(|handler| {
            handler
                .parser_proxies
                .as_ref()
                .and_then(|proxy| Some(&*proxy.parser))
        });
        match lib.get::<*mut Option<&dyn InstructionParser>>(b"INTERFACE\0") {
            Ok(res) => {
                res.write(proxy);
            }
            Err(err) => {
                log::info!("get INTERFACE err: {:?}", err);
            }
        }
        let adapter_decl = lib
            .get::<*mut AdapterDeclaration>(b"adapter_declaration\0")?
            .read();
        let mut registrar = IndexerHandler::new(lib);
        (adapter_decl.register)(&mut registrar);
        self.indexer_handler = Some(registrar);
        Ok(())
    }
    async fn start_mapping(&mut self) -> Result<(), Box<dyn Error>> {
        let mut handler_proxy = self
            .indexer_handler
            .as_ref()
            .and_then(|adapter| adapter.handler_proxies.clone());
        let indexer_hash = self.indexer.got_block.clone();
        if let Some(proxy) = handler_proxy {
            loop {
                let blocks = self.block_buffer.read_blocks(&self.got_block);
                let size = blocks.len();
                log::info!(
                    "Indexer {:?} got {:?} blocks from buffer.",
                    &self.indexer.hash,
                    size
                );
                if size > 0 {
                    let now = Instant::now();
                    let vec_blocks = blocks
                        .iter()
                        .map(|block| (**block).clone())
                        .collect::<Vec<SolanaBlock>>();
                    match proxy.handle_blocks(&vec_blocks) {
                        Err(err) => {
                            log::error!("{} Error while handle received message", err);
                        }
                        Ok(block_slot) => {
                            self.got_block = Some(block_slot as Slot);
                            log::info!(
                                "Indexer {:?} process {:?} received blocks in {:?}",
                                &self.indexer.hash,
                                size,
                                now.elapsed()
                            );
                        }
                    }
                } else {
                    sleep(Duration::from_millis(
                        WAITING_FOR_INCOMING_BLOCK_MILLISECOND,
                    ))
                    .await;
                }
            }
        }
        Ok(())
    }
    async fn start_mapping_seperated_stream(
        &mut self,
        //store: Arc<Mutex<Box<&mut dyn IndexStore>>>,
    ) -> Result<(), Box<dyn Error>> {
        let mut handler_proxy = self
            .indexer_handler
            .as_ref()
            .and_then(|adapter| adapter.handler_proxies.clone());
        let data_source = self.manifest.data_sources.get(0).unwrap().clone();
        if let Some(proxy) = handler_proxy {
            let mut opt_stream: Option<Streaming<BlockResponse>> = None;
            let mut start_block = if self.indexer.got_block >= 0 {
                Some(self.indexer.got_block.clone() as u64 + 1)
            } else {
                None
            };
            let (history_block_tx, mut history_block_rx) = mpsc::channel::<Vec<SolanaBlock>>(64);
            let mut getting_history_blocks = false;
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
                            .try_create_block_stream(&data_source, start_block.clone())
                            .await;
                        if opt_stream.is_none() {
                            //Sleep for a while and reconnect
                            sleep(Duration::from_secs(GET_STREAM_TIMEOUT_SEC)).await;
                        }
                    }
                    Some(ref mut stream) => {
                        let response =
                            timeout(Duration::from_secs(GET_BLOCK_TIMEOUT_SEC), stream.message())
                                .await;
                        match response {
                            Ok(Ok(res)) => {
                                if let Some(mut data) = res {
                                    let now = Instant::now();
                                    let blocks: Vec<SolanaBlock> =
                                        serde_json::from_slice(&mut data.payload).unwrap();
                                    log::info!(
                                        "Deserialization time of {:? }blocks: {:?}",
                                        blocks.len(),
                                        now.elapsed()
                                    );
                                    //Get history block from first transaction in first block
                                    if let Some(block) = blocks.get(0) {
                                        //Open history thread for the first detection
                                        if !getting_history_blocks
                                            && block.block.parent_slot
                                                > self.indexer.got_block as u64
                                        {
                                            getting_history_blocks = true;
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

                                    let now = Instant::now();
                                    match proxy.handle_blocks(&blocks) {
                                        Err(err) => {
                                            log::error!(
                                                "{} Error while handle received message",
                                                err
                                            );
                                        }
                                        Ok(last_block_slot) => {
                                            log::info!(
                                                "Process {:?} received blocks in {:?}",
                                                blocks.len(),
                                                now.elapsed()
                                            );
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
        Ok(())
    }
    /// Collect history blocks in range [from_block, to_block)
    async fn load_history_data(
        &mut self,
        sender: Sender<Vec<SolanaBlock>>,
        from_block: u64,
        last_signature: Option<Signature>,
    ) {
        //******************* Backward check ***************************//
        info!(
            "Start get transaction backward with filter address: {:?}",
            &self.indexer
        );
        let chain_adapters = self.network_adapters.clone();
        let address = self
            .manifest
            .data_sources
            .get(0)
            .unwrap()
            .source
            .address
            .as_ref()
            .and_then(|addr| Pubkey::from_str(addr.as_str()).ok());
        //Create a cloned reference to proxy for sub thread
        let proxy = self
            .indexer_handler
            .as_ref()
            .unwrap()
            .handler_proxies
            .as_ref()
            .unwrap()
            .clone();
        tokio::spawn(async move {
            if let Some(pubkey) = address {
                let mut adapters = chain_adapters.lock().unwrap();
                let signatures =
                    adapters.get_signatures_for_address(&pubkey, Some(from_block), last_signature);
                //******************* Forward run ***************************//
                info!("Start get {} transaction forward.", signatures.len());
                //Get all transactions by history signatures
                let mut adapters = chain_adapters.lock().unwrap();
                let confirmed_blocks = adapters.get_confirmed_blocks(&signatures);
                let ext_blocks = confirmed_blocks
                    .into_iter()
                    .filter_map(|confirmed_block| {
                        let block_number = confirmed_block.block_slot;
                        confirmed_block.block.and_then(|block| {
                            Some(ExtBlock {
                                version: "".to_string(),
                                timestamp: 0,
                                block_number,
                                block,
                                list_log_messages: vec![],
                            })
                        })
                    })
                    .collect();
                sender.send(ext_blocks);
                //proxy.handle_blocks(&ext_blocks);
            }
        });
    }

    async fn try_create_block_stream(
        &self,
        data_source: &DataSource,
        start_block: Option<u64>,
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
        log::info!(
            "Indexer {:?} get new stream from block {:?}.",
            &self.indexer.name,
            &start_block
        );
        let chain_type = match data_source.kind.split('/').next().unwrap() {
            "ethereum" => ChainType::Ethereum,
            _ => ChainType::Solana, // If not provided, assume it's Solana network
        };
        let transaction_request = BlockRequest {
            indexer_hash: self.indexer.hash.clone(),
            start_block_number: start_block,
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
}
