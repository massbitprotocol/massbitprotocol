//use super::ipfs::create_ipfs_clients;
use crate::setting::*;
pub use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
pub use crate::{HandlerProxyType, PluginRegistrar, WasmHandlerProxyType};
use futures::future;
use graph::components::metrics::stopwatch::StopwatchMetrics;
use graph::prelude::{
    DeploymentHash, HostMetrics, LinkResolver as LinkResolverTrait, MetricsRegistry,
};

use graph::blockchain::{BlockHash, BlockPtr, DataSource as _};
use graph::blockchain::{
    DataSource as DataSourceTrait, HostFn, RuntimeAdapter as RuntimeAdapterTrait,
};
use graph::components::store::{ModificationsAndCache, StoreError, WritableStore};
use graph::components::subgraph::BlockState;
use graph::data::subgraph::{Mapping, SubgraphManifest, TemplateSource};
use graph::prelude::CheapClone;
use graph::util::lfu_cache::LfuCache;
use graph_chain_ethereum::Chain;
use graph_chain_ethereum::{trigger::MappingTrigger, DataSource, DataSourceTemplate};
use graph_core::LinkResolver;
//use graph_runtime_wasm::{ExperimentalFeatures, HostExports, MappingContext};
use graph::tokio_stream::StreamExt;
use graph_mock::MockMetricsRegistry;
use graph_runtime_wasm::ValidModule;
use index_store::core::Store;
use lazy_static::lazy_static;
use libloading::Library;
use log::info;
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, SolanaEncodedBlock,
    SolanaLogMessages, SolanaTransaction,
};
use massbit_common::prelude::anyhow::{self, Context};
use massbit_common::prelude::tokio::io::AsyncReadExt;
use massbit_runtime_wasm::host_exports::create_ethereum_call;
use massbit_runtime_wasm::manifest::datasource::*;
use massbit_runtime_wasm::mapping::FromFile;
use massbit_runtime_wasm::prelude::{Logger, Version};
use massbit_runtime_wasm::store::postgres::store_builder::*;
use massbit_runtime_wasm::store::PostgresIndexStore;
use massbit_runtime_wasm::{slog, store};
use massbit_runtime_wasm::{HostExports, MappingContext, WasmInstance};
use serde_yaml::Value;
use slog::o;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;
use std::{
    alloc::System, collections::HashMap, env, error::Error, ffi::OsStr, fmt, path::PathBuf,
    sync::Arc,
};
use tonic::{Request, Streaming};

const API_VERSION_0_0_4: Version = Version::new(0, 0, 4);
const API_VERSION_0_0_5: Version = Version::new(0, 0, 5);
lazy_static! {
    static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
    static ref GENERATED_FOLDER: String = String::from("index-manager/generated/");
    static ref COMPONENT_NAME: String = String::from("[Adapter-Manager]");
}
#[global_allocator]
static ALLOCATOR: System = System;

#[derive(Copy, Clone)]
pub struct AdapterDeclaration {
    pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
}
//adapter_type => HandlerProxyType

pub struct AdapterHandler {
    indexer_hash: String,
    pub lib: Arc<Library>,
    pub handler_proxies: HashMap<String, HandlerProxyType>,
}

impl AdapterHandler {
    fn new(hash: String, lib: Arc<Library>) -> AdapterHandler {
        AdapterHandler {
            indexer_hash: hash,
            lib,
            handler_proxies: HashMap::default(),
        }
    }
}

pub struct WasmAdapter {
    indexer_hash: String,
    pub wasm: Arc<ValidModule>,
    pub handler_proxies: HashMap<String, Arc<Option<WasmHandlerProxyType>>>,
}

impl WasmAdapter {
    fn new(hash: String, wasm: Arc<ValidModule>) -> WasmAdapter {
        WasmAdapter {
            indexer_hash: hash,
            wasm,
            handler_proxies: HashMap::default(),
        }
    }
}
/*
pub struct AdapterManager<'a> {
    pub store: &'a dyn Store,
    pub libs: Vec<Rc<Library>>,
    handler_proxies: MapProxies,
}
impl<'a> AdapterManager<'a> {
    pub fn new(store: &mut dyn Store) -> AdapterManager {
        AdapterManager {
            store,
            libs: vec![],
            handler_proxies: HashMap::default(),
        }
    }
}
*/

pub struct AdapterManager {
    store: Option<PostgresIndexStore>,
    libs: HashMap<String, Arc<Library>>,
    map_handlers: HashMap<String, AdapterHandler>,
}

impl AdapterManager {
    pub fn new() -> AdapterManager {
        AdapterManager {
            store: None,
            libs: HashMap::default(),
            map_handlers: HashMap::default(),
        }
    }
    pub async fn init(
        &mut self,
        hash: &String,
        config: &Value,
        mapping: &PathBuf,
        schema: &PathBuf,
        manifest: &Option<SubgraphManifest<Chain>>,
    ) -> Result<(), Box<dyn Error>> {
        /*
        let templates = match config["templates"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|tpl| {
                    DataSourceTemplate::try_from(tpl)
                        .with_context(|| {
                            format!(
                                "Failed to create datasource from value `{:?}`, invalid address provided",
                                tpl
                            )
                        })
                        .unwrap()
                })
                .collect::<Vec<DataSourceTemplate>>(),
            _ => Vec::default(),
        };
         */
        /*
        //Inject static wasm module
        let quickswap_path = "/home/viettai/Massbit/QuickSwap-subgraph/build/";
        let factory = "Factory/Factory.wasm";
        let pair = "templates/Pair/Pair.wasm";
        let factory_wasm = self
            .load_wasm_content(format!("{}/{}", quickswap_path, factory))
            .await;
        let pair_wasm = self
            .load_wasm_content(format!("{}/{}", quickswap_path, pair))
            .await;
        */
        let mut empty_ds: Vec<DataSource> = vec![];
        let mut data_sources: Vec<DataSource> = vec![];
        let mut templates: Vec<DataSourceTemplate> = vec![];
        if let Some(sgd) = manifest {
            data_sources = sgd
                .data_sources
                .iter()
                .map(|ds| ds.clone())
                .collect::<Vec<DataSource>>();
            templates = sgd
                .templates
                .iter()
                .map(|tpl| tpl.clone())
                .collect::<Vec<DataSourceTemplate>>();
        }
        /*
        if let Some(template) = templates.get_mut(0) {
            template.mapping.runtime = Arc::new(pair_wasm);
        }
        */
        /*
        let (data_sources, templates): (&mut Vec<DataSource>, Vec<DataSourceTemplate>) =
            match manifest {
                Some(sgd) => (
                    &sgd.data_sources.clone(),
                    sgd.templates
                        .iter()
                        .map(|tpl| tpl.clone())
                        .collect::<Vec<DataSourceTemplate>>(),
                ),
                None => (&mut empty_ds, vec![]),
            };
        */
        //println!("{:?}", data_sources);
        //println!("{:?}", templates);

        let arc_templates = Arc::new(templates);
        //Todo: Currently adapter only works with one datasource
        /*
        assert_eq!(
            data_sources.len(),
            1,
            "Error: Number datasource: {} is not 1.",
            data_sources.len()
        );
         */
        match data_sources.get(0) {
            Some(data_source) => {
                info!("Data source: {:?}", data_source);
                let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone()).await?;
                log::info!(
                    "{} Init Streamout client for chain {}",
                    &*COMPONENT_NAME,
                    &data_source.kind
                );
                let chain_type = get_chain_type(data_source);
                let get_blocks_request = GetBlocksRequest {
                    start_block_number: 0,
                    end_block_number: 1,
                    chain_type: chain_type as i32,
                };
                let mut stream: Streaming<GenericDataProto> = client
                    .list_blocks(Request::new(get_blocks_request))
                    .await?
                    .into_inner();

                match data_source.mapping.language.as_str() {
                    "wasm/assemblyscript" => {
                        self.handle_wasm_mapping(
                            hash,
                            data_source,
                            arc_templates,
                            mapping,
                            schema,
                            &mut stream,
                        )
                        .await
                    }
                    //Default use rust
                    _ => {
                        self.handle_rust_mapping(hash, data_source, mapping, &mut stream)
                            .await
                    }
                }
            }
            _ => {
                /*
                let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone()).await?;
                log::info!(
                    "{} Init Streamout client for chain {}",
                    &*COMPONENT_NAME,
                    &data_source.kind
                );
                let chain_type = get_chain_type(data_source);
                let get_blocks_request = GetBlocksRequest {
                    start_block_number: 0,
                    end_block_number: 1,
                    chain_type: chain_type as i32,
                };
                let mut stream: Streaming<GenericDataProto> = client
                    .list_blocks(Request::new(get_blocks_request))
                    .await?
                    .into_inner();
                self.handle_rust_mapping(hash, config, mapping, &mut stream)
                    .await
                 */
                Ok(())
            }
        }
    }
    pub async fn load_wasm_content(&self, path: String) -> Vec<u8> {
        let mut content = Vec::new();
        let mut file = File::open(&path).expect("Unable to open file");
        file.read_to_end(&mut content)
            .expect("Unable to read file content");
        content
    }
    pub async fn init0(
        &mut self,
        hash: &String,
        config: &Value,
        mapping: &PathBuf,
        schema: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        let data_sources = match config["dataSources"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|datasource| {
                    DataSource::try_from(datasource)
                        .with_context(|| {
                            format!(
                                "Failed to create datasource from value `{:?}`, invalid address provided",
                                datasource
                            )
                        })
                        .unwrap()
                })
                .collect::<Vec<DataSource>>(),
            _ => Vec::default(),
        };
        let templates = match config["templates"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|tpl| {
                    DataSourceTemplate::try_from(tpl)
                        .with_context(|| {
                            format!(
                                "Failed to create datasource from value `{:?}`, invalid address provided",
                                tpl
                            )
                        })
                        .unwrap()
                })
                .collect::<Vec<DataSourceTemplate>>(),
            _ => Vec::default(),
        };
        let templates = Arc::new(templates);
        //Todo: Currently adapter only works with one data_source
        match data_sources.get(0) {
            Some(data_source) => {
                let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone()).await?;
                log::info!(
                    "{} Init Streamout client for chain {}",
                    &*COMPONENT_NAME,
                    &data_source.kind
                );
                let chain_type = get_chain_type(data_source);
                let get_blocks_request = GetBlocksRequest {
                    start_block_number: 0,
                    end_block_number: 1,
                    chain_type: chain_type as i32,
                };
                let mut stream: Streaming<GenericDataProto> = client
                    .list_blocks(Request::new(get_blocks_request))
                    .await?
                    .into_inner();
                log::info!(
                    "{} Detect mapping language {}",
                    &*COMPONENT_NAME,
                    &data_source.mapping.language
                );
                match data_source.mapping.language.as_str() {
                    "wasm/assemblyscript" => {
                        self.handle_wasm_mapping(
                            hash,
                            data_source,
                            templates,
                            mapping,
                            schema,
                            &mut stream,
                        )
                        .await
                    }
                    //Default use rust
                    _ => {
                        self.handle_rust_mapping(hash, data_source, mapping, &mut stream)
                            .await
                    }
                }
            }
            _ => Ok(()),
        }
    }
    async fn handle_wasm_mapping<P: AsRef<Path>>(
        &mut self,
        indexer_hash: &String,
        data_source: &DataSource,
        templates: Arc<Vec<DataSourceTemplate>>,
        mapping_path: P,
        schema_path: P,
        stream: &mut Streaming<GenericDataProto>,
    ) -> Result<(), Box<dyn Error>> {
        /*
        log::info!("Load wasm file from {:?}", mapping_path.as_ref());
        let valid_module = Arc::new(ValidModule::from_file(mapping_path.as_ref()).unwrap());
        log::info!(
            "Import wasm file {:?} successfully with modules {:?}",
            mapping_path.as_ref(),
            &valid_module.import_name_to_modules
        );
         */
        let store =
            Arc::new(StoreBuilder::create_store(indexer_hash.as_str(), &schema_path).unwrap());
        let registry = Arc::new(MockMetricsRegistry::new());
        /*
        let start = Instant::now();
        let clone_store = Arc::clone(&store);
        let mut wasm_instance = self
            .load_wasm(
                indexer_hash,
                data_source,
                clone_store,
                Arc::clone(&valid_module),
            )
            .unwrap();
        log::info!(
            "{} Create wasm instance finished in {:?}",
            &*COMPONENT_NAME,
            start.elapsed()
        );
        */
        log::info!("{} Start mapping using wasm binary", &*COMPONENT_NAME);
        let adapter_name = data_source
            .kind
            .split("/")
            .collect::<Vec<&str>>()
            .get(0)
            .unwrap()
            .to_string();
        let mut handler_proxy = WasmHandlerProxyType::create_proxy(
            &adapter_name,
            indexer_hash,
            store,
            data_source.clone(), //Arc::clone(&valid_module),
            templates,
        );
        /*
        let mut wasm_adapter = WasmAdapter::new(indexer_hash.clone(), Arc::clone(&valid_module));
        wasm_adapter
            .handler_proxies
            .insert(adapter_name.clone(), Arc::clone(&handler_proxy));
         */
        let stopwatch = StopwatchMetrics::new(
            Logger::root(slog::Discard, o!()),
            DEPLOYMENT_HASH.cheap_clone(),
            registry.clone(),
        );
        while let Some(mut data) = stream.message().await? {
            let data_type = DataType::from_i32(data.data_type).unwrap();
            let block_ptr_to = BlockPtr {
                hash: BlockHash(data.block_hash.as_bytes().into()),
                number: data.block_number as i32,
            };
            log::info!(
                "{} Chain {:?} received data block = {:?}, hash = {:?}, data type = {:?}",
                &*COMPONENT_NAME,
                ChainType::from_i32(data.chain_type).unwrap(),
                data.block_number,
                data.block_hash,
                data_type
            );
            /*
            let start = Instant::now();
            let mut wasm_instance = self
                .load_wasm(
                    indexer_hash,
                    data_source,
                    templates.clone(),
                    store.clone(),
                    //valid_module.cheap_clone(),
                    Arc::clone(&valid_module),
                    registry.clone(),
                    //link_resolver.clone(),
                )
                .unwrap();
            log::info!(
                "{} Create wasm instance finished in {:?}",
                &*COMPONENT_NAME,
                start.elapsed()
            );
             */
            //let clone_proxy = Arc::clone(&handler_proxy);
            if let Some(ref mut proxy) = handler_proxy {
                match proxy.handle_wasm_mapping(&mut data) {
                    Err(err) => {
                        log::error!("{} Error while handle received message", err);
                    }
                    _ => {}
                }
                /*
                match proxy.handle_wasm_mapping(&mut wasm_instance, data_source, &mut data) {
                    Err(err) => {
                        log::error!("{} Error while handle received message", err);
                    }
                    _ => {}
                }
                */
                log::info!("Finish wasm mapping");

                /*
                let state = wasm_instance.take_ctx().ctx.state;
                let ModificationsAndCache {
                    modifications: mods,
                    data_sources,
                    entity_lfu_cache: cache,
                } = state.entity_cache.as_modifications().map_err(|e| {
                    log::error!("Error {:?}", e);
                    StoreError::Unknown(e.into())
                })?;

                // Transact entity modifications into the store
                if mods.len() > 0 {
                    store.transact_block_operations(
                        block_ptr_to,
                        mods,
                        stopwatch.cheap_clone(),
                        data_sources,
                        vec![],
                    );
                }
                 */
                //.map(move |_| {
                //    metrics.transaction.update_duration(started.elapsed());
                //    block_ptr
                //});
                /*
                future::result(
                    store
                        .transact_block_operations(
                            block_ptr.clone(),
                            modifications,
                            stopwatch,
                            Vec::new(),
                            Vec::new(),
                        )
                        .map_err(|e| e.into())
                        .map(move |_| {
                            metrics.transaction.update_duration(started.elapsed());
                            block_ptr
                        }),
                )
                 */
            }
        }
        Ok(())
    }

    pub fn load_wasm(
        &mut self,
        indexer_hash: &String,
        data_source: &DataSource,
        templates: Arc<Vec<DataSourceTemplate>>,
        store: Arc<dyn WritableStore>,
        valid_module: Arc<ValidModule>,
        registry: Arc<MockMetricsRegistry>,
        block_ptr: BlockPtr,
        //link_resolver: Arc<dyn LinkResolverTrait>,
    ) -> Result<WasmInstance<Chain>, anyhow::Error> {
        let api_version = API_VERSION_0_0_4.clone();
        let stopwatch_metrics = StopwatchMetrics::new(
            Logger::root(slog::Discard, o!()),
            DeploymentHash::new("_indexer").unwrap(),
            registry.clone(),
        );
        let host_metrics = Arc::new(HostMetrics::new(
            registry.clone(),
            indexer_hash.as_str(),
            stopwatch_metrics,
        ));
        /*
        let templates = vec![DataSourceTemplate {
            kind: data_source.kind.clone(),
            name: data_source.name.clone(),
            network: data_source.network.clone(),
            source: TemplateSource {
                abi: data_source.source.abi.clone(),
            },
            mapping: Mapping {
                kind: data_source.mapping.kind.clone(),
                api_version: api_version.clone(),
                language: data_source.mapping.language.clone(),
                entities: vec![],
                abis: vec![],
                event_handlers: vec![],
                call_handlers: vec![],
                block_handlers: vec![],
                runtime: Arc::new(vec![]),
                //link: Default::default(),
            },
        }];
        */
        let network = match &data_source.network {
            None => String::from("ethereum"),
            Some(val) => val.clone(),
        };
        /*
        //graph HostExports
        let host_exports = HostExports::new(
            DeploymentHash::new(indexer_hash.clone()).unwrap(),
            data_source,
            network,
            Arc::clone(&templates),
            link_resolver,
            //Arc::new(graph_core::LinkResolver::from(IpfsClient::localhost())),
            Arc::new(SubgraphStore::new()),
        );
         */
        let host_exports = HostExports::new(
            indexer_hash.as_str(),
            data_source,
            network,
            Arc::clone(&templates),
            api_version,
        );
        //let store = store::IndexStore::new();
        //check if wasm module use import ethereum.call
        let host_fns: Vec<HostFn> = match valid_module.import_name_to_modules.get("ethereum.call") {
            None => Vec::new(),
            Some(_) => {
                vec![create_ethereum_call(data_source)]
                //vec![create_mock_ethereum_call(data_source)]
            }
        };
        //data_source.mapping.requires_archive();
        let context = MappingContext {
            logger: Logger::root(slog::Discard, o!()),
            block_ptr,
            host_exports: Arc::new(host_exports),
            //state: IndexerState::new(store, Default::default()),
            state: BlockState::new(store, Default::default()),
            //proof_of_indexing: None,
            host_fns: Arc::new(host_fns),
        };
        let timeout = None;

        WasmInstance::from_valid_module_with_ctx(
            valid_module,
            context,
            host_metrics,
            timeout,
            //ExperimentalFeatures {
            //    allow_non_deterministic_ipfs: false,
            //},
        )
    }

    async fn handle_rust_mapping<P: AsRef<OsStr>>(
        &mut self,
        indexer_hash: &String,
        data_source: &DataSource,
        mapping_path: P,
        stream: &mut Streaming<GenericDataProto>,
    ) -> Result<(), Box<dyn Error>> {
        //let store = PostgresIndexStore::new(DATABASE_CONNECTION_STRING.as_str()).await;
        let empty_path = PathBuf::new();
        let store = StoreBuilder::create_store(indexer_hash.as_str(), &empty_path).unwrap();
        self.store = Some(store);
        unsafe {
            match self.load(indexer_hash, mapping_path).await {
                Ok(_) => log::info!("{} Load library successfully", &*COMPONENT_NAME),
                Err(err) => println!("Load library with error {:?}", err),
            }
        }
        log::info!("{} Start mapping using rust", &*COMPONENT_NAME);
        let adapter_name = data_source.kind.as_str();
        if let Some(adapter_handler) = self.map_handlers.get(indexer_hash.as_str()) {
            if let Some(handler_proxy) = adapter_handler.handler_proxies.get(adapter_name) {
                while let Some(data) = stream.message().await? {
                    let mut data = data as GenericDataProto;
                    log::info!(
                        "{} Chain {:?} received data block = {:?}, hash = {:?}, data type = {:?}",
                        &*COMPONENT_NAME,
                        ChainType::from_i32(data.chain_type).unwrap(),
                        data.block_number,
                        data.block_hash,
                        DataType::from_i32(data.data_type).unwrap()
                    );
                    match handler_proxy.handle_rust_mapping(&mut data) {
                        Err(err) => {
                            log::error!("{} Error while handle received message", err);
                        }
                        _ => {}
                    }
                }
            } else {
                log::debug!(
                    "{} Cannot find proxy for adapter {}",
                    *COMPONENT_NAME,
                    adapter_name
                );
            }
        } else {
            log::debug!(
                "{} Cannot find adapter handler for indexer {}",
                &*COMPONENT_NAME,
                &indexer_hash
            );
        }
        Ok(())
    }
    /// Load a plugin library
    /// A plugin library **must** be implemented using the
    /// [`model::adapter_declaration!()`] macro. Trying manually implement
    /// a plugin without going through that macro will result in undefined
    /// behaviour.
    pub async unsafe fn load<P: AsRef<OsStr>>(
        &mut self,
        indexer_hash: &String,
        library_path: P,
    ) -> Result<(), Box<dyn Error>> {
        let lib = Arc::new(Library::new(library_path)?);
        // inject store to plugin
        let store = &mut self.store;
        match store {
            Some(store) => {
                lib.get::<*mut Option<&dyn Store>>(b"STORE\0")?
                    .write(Some(store));
            }
            _ => {}
        }
        let adapter_decl = lib
            .get::<*mut AdapterDeclaration>(b"adapter_declaration\0")?
            .read();
        let mut registrar = AdapterHandler::new(indexer_hash.clone(), Arc::clone(&lib));
        (adapter_decl.register)(&mut registrar);
        self.map_handlers.insert(indexer_hash.clone(), registrar);
        self.libs.insert(indexer_hash.clone(), lib);
        Ok(())
    }
}

// General trait for handling message,
// every adapter proxies must implement this trait
pub trait MessageHandler {
    fn handle_rust_mapping(&self, message: &mut GenericDataProto) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn handle_wasm_mapping(
        &mut self,
        //wasm_instance: &mut WasmInstance<Chain>,
        //data_source: &DataSource,
        message: &mut GenericDataProto,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct AdapterError(String);

impl AdapterError {
    pub fn new(msg: &str) -> AdapterError {
        AdapterError(msg.to_string())
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Error for AdapterError {
    fn description(&self) -> &str {
        &self.0
    }
}
