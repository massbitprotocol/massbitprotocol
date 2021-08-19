use crate::setting::*;
pub use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
pub use crate::{HandlerProxyType, PluginRegistrar, WasmHandlerProxyType};
use index_store::core::Store;
use lazy_static::lazy_static;
use libloading::Library;
use massbit_runtime_wasm::chain::ethereum::Chain;
use massbit_runtime_wasm::graph::components::metrics::stopwatch::StopwatchMetrics;
use massbit_runtime_wasm::graph::HostMetrics;
use massbit_runtime_wasm::host_exports::HostExports;
use massbit_runtime_wasm::indexer::manifest::{Mapping, MappingBlockHandler};
use massbit_runtime_wasm::indexer::types::BlockPtr;
use massbit_runtime_wasm::indexer::{manifest, IndexerState};
use massbit_runtime_wasm::store::IndexStore;
//use massbit_runtime_wasm::manifest::{DataSource, DataSourceTemplate, Mapping, TemplateSource};
//use massbit_chain_ethereum::data_type::{decode, EthereumBlock, EthereumTransaction};
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode as solana_decode, SolanaEncodedBlock,
    SolanaLogMessages, SolanaTransaction,
};
use massbit_runtime_wasm::chain::ethereum::data_source::{DataSource, DataSourceTemplate};
use massbit_runtime_wasm::chain::ethereum::trigger::MappingTrigger;
use massbit_runtime_wasm::graph::components::store::{
    EntityKey, EntityType, StoreError, WritableStore,
};
use massbit_runtime_wasm::graph::data::query::error::QueryExecutionError;
use massbit_runtime_wasm::graph::data::store::Entity;
use massbit_runtime_wasm::indexer::blockchain::DataSource as DataSourceTrait;
use massbit_runtime_wasm::indexer::manifest::TemplateSource;
use massbit_runtime_wasm::mapping::{MappingContext, ValidModule};
use massbit_runtime_wasm::mock::MockMetricsRegistry;
use massbit_runtime_wasm::module::WasmInstance;
use massbit_runtime_wasm::prelude::anyhow::Context;
use massbit_runtime_wasm::prelude::serde::__private::TryFrom;
use massbit_runtime_wasm::prelude::{anyhow, Logger, Version};
use massbit_runtime_wasm::{slog, store};
use serde_yaml::Value;
use slog::o;
use std::collections::BTreeMap;
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
    static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
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
    store: Option<IndexStore>,
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
        //Todo: Currently adapter only works with one datasource
        match data_sources.get(0) {
            Some(datasource) => {
                let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone()).await?;
                log::info!(
                    "{} Init Streamout client for chain {}",
                    &*COMPONENT_NAME,
                    &datasource.kind
                );
                let chain_type = get_chain_type(datasource);
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
                    &datasource.mapping.language
                );
                match datasource.mapping.language.as_str() {
                    "wasm/assemblyscript" => {
                        self.handle_wasm_mapping(hash, datasource, mapping, &mut stream)
                            .await
                    }
                    //Default use rust
                    _ => {
                        self.handle_rust_mapping(hash, datasource, mapping, &mut stream)
                            .await
                    }
                }
            }
            _ => Ok(()),
        }
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
    pub fn load_wasm(
        &mut self,
        indexer_hash: &String,
        datasource: &DataSource,
        store: Arc<dyn WritableStore>,
        valid_module: Arc<ValidModule>,
    ) -> Result<WasmInstance<Chain>, anyhow::Error> {
        let api_version = API_VERSION_0_0_4.clone();
        let metrics_registry = Arc::new(MockMetricsRegistry::new());
        let stopwatch_metrics = StopwatchMetrics::new(
            Logger::root(slog::Discard, o!()),
            indexer_hash.clone(),
            metrics_registry.clone(),
        );
        let host_metrics = Arc::new(HostMetrics::new(
            metrics_registry,
            indexer_hash.as_str(),
            stopwatch_metrics,
        ));
        let templates = vec![DataSourceTemplate {
            kind: datasource.kind.clone(),
            name: datasource.name.clone(),
            network: datasource.network.clone(),
            source: TemplateSource {
                abi: datasource.source.abi.clone(),
            },
            mapping: Mapping {
                kind: datasource.mapping.kind.clone(),
                api_version: api_version.clone(),
                language: datasource.mapping.language.clone(),
                entities: vec![],
                abis: vec![],
                event_handlers: vec![],
                call_handlers: vec![],
                block_handlers: vec![],
                runtime: Arc::new(vec![]),
                //link: Default::default(),
            },
        }];

        let network = datasource.network.clone().unwrap();
        let host_exports = HostExports::new(
            indexer_hash.as_str(),
            datasource,
            network,
            Arc::new(templates),
            api_version,
            //Arc::new(graph_core::LinkResolver::from(IpfsClient::localhost())),
            //store,
        );
        //let store = store::IndexStore::new();
        let context = MappingContext {
            logger: Logger::root(slog::Discard, o!()),
            block_ptr: BlockPtr {
                hash: Default::default(),
                number: datasource.source.start_block,
            },
            host_exports: Arc::new(host_exports),
            state: IndexerState::new(Arc::clone(&store), Default::default()),
            /*
            state: BlockState::new(store.writable(&deployment).unwrap(), Default::default()),
            proof_of_indexing: None,
            host_fns: Arc::new(Vec::new()),
             */
        };
        let timeout = None;
        WasmInstance::from_valid_module_with_ctx(
            valid_module,
            context,
            host_metrics,
            timeout,
            //experimental_features,
        )
    }
    async fn handle_wasm_mapping<P: AsRef<OsStr>>(
        &mut self,
        indexer_hash: &String,
        datasource: &DataSource,
        mapping_path: P,
        stream: &mut Streaming<GenericDataProto>,
    ) -> Result<(), Box<dyn Error>> {
        log::info!("Load wasm file from {:?}", mapping_path.as_ref());
        let valid_module = Arc::new(ValidModule::from_file(mapping_path.as_ref()).unwrap());
        let mut wasm_adapter = WasmAdapter::new(indexer_hash.clone(), Arc::clone(&valid_module));
        log::info!(
            "Import wasm file {:?} successfully with modules {:?}",
            mapping_path.as_ref(),
            &valid_module.import_name_to_modules
        );
        let store = Arc::new(IndexStore::new(DATABASE_CONNECTION_STRING.as_str()).await);
        /*
        let mut wasm_instance = self
            .load_wasm(indexer_hash, datasource, Arc::clone(&valid_module))
            .unwrap();
        */
        log::info!("{} Start mapping using wasm binary", &*COMPONENT_NAME);
        let adapter_name = datasource
            .kind
            .split("/")
            .collect::<Vec<&str>>()
            .get(0)
            .unwrap()
            .to_string();
        let handler_proxy = Arc::new(WasmHandlerProxyType::create_proxy(
            &adapter_name,
            Arc::clone(&valid_module),
        ));
        wasm_adapter
            .handler_proxies
            .insert(adapter_name.clone(), Arc::clone(&handler_proxy));
        let mapping: &Mapping = datasource.mapping();
        while let Some(mut data) = stream.message().await? {
            let data_type = DataType::from_i32(data.data_type).unwrap();
            log::info!(
                "{} Chain {:?} received data block = {:?}, hash = {:?}, data type = {:?}",
                &*COMPONENT_NAME,
                ChainType::from_i32(data.chain_type).unwrap(),
                data.block_number,
                data.block_hash,
                data_type
            );

            let start = Instant::now();
            let clone_store = Arc::clone(&store);
            let mut wasm_instance = self
                .load_wasm(
                    indexer_hash,
                    datasource,
                    clone_store,
                    Arc::clone(&valid_module),
                )
                .unwrap();
            log::info!(
                "{} Create wasm instance finished in {:?}",
                &*COMPONENT_NAME,
                start.elapsed()
            );
            let clone_proxy = Arc::clone(&handler_proxy);
            if let Some(proxy) = &*clone_proxy {
                match proxy.handle_wasm_mapping(&mut wasm_instance, mapping, &mut data) {
                    Err(err) => {
                        log::error!("{} Error while handle received message", err);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    async fn handle_rust_mapping<P: AsRef<OsStr>>(
        &mut self,
        indexer_hash: &String,
        datasource: &DataSource,
        mapping_path: P,
        stream: &mut Streaming<GenericDataProto>,
    ) -> Result<(), Box<dyn Error>> {
        let store = IndexStore::new(DATABASE_CONNECTION_STRING.as_str()).await;
        self.store = Some(store);
        unsafe {
            match self.load(indexer_hash, mapping_path).await {
                Ok(_) => log::info!("{} Load library successfully", &*COMPONENT_NAME),
                Err(err) => println!("Load library with error {:?}", err),
            }
        }
        log::info!("{} Start mapping using rust", &*COMPONENT_NAME);
        let adapter_name = datasource.kind.as_str();
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
}

// General trait for handling message,
// every adapter proxies must implement this trait
pub trait MessageHandler {
    fn handle_rust_mapping(&self, message: &mut GenericDataProto) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn handle_wasm_mapping(
        &self,
        wasm_instance: &mut WasmInstance<Chain>,
        mapping: &Mapping,
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
