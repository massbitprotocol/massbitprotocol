use crate::setting::*;
pub use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
pub use crate::{HandlerProxyType, PluginRegistrar, WasmHandlerProxyType};
use graph::data::subgraph::SubgraphManifest;
use graph_chain_ethereum::Chain;
use graph_chain_ethereum::{DataSource, DataSourceTemplate};
use graph_runtime_wasm::ValidModule;
use index_store::postgres::store_builder::*;
use index_store::{IndexerState, Store};
use lazy_static::lazy_static;
use libloading::Library;
use massbit_common::prelude::tokio::time::sleep;
use serde_yaml::Value;
use std::path::Path;
use std::{
    alloc::System, collections::HashMap, env, error::Error, ffi::OsStr, fmt, path::PathBuf,
    sync::Arc,
    time::{Instant, Duration}
};
use tonic::{Request, Streaming};

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
    //store: Option<dyn Store>,
    libs: HashMap<String, Arc<Library>>,
    map_handlers: HashMap<String, AdapterHandler>,
}

impl AdapterManager {
    pub fn new() -> AdapterManager {
        AdapterManager {
            //store: None,
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

        let arc_templates = Arc::new(templates);
        //Todo: Currently adapter only works with one datasource
        assert_eq!(
            data_sources.len(),
            1,
            "Error: Number datasource: {} is not 1.",
            data_sources.len()
        );
        match data_sources.get(0) {
            Some(data_source) => {
                log::info!(
                    "{} Init Streamout client for chain {} using language {}",
                    &*COMPONENT_NAME,
                    &data_source.kind,
                    &data_source.mapping.language
                );
                loop {
                    sleep(Duration::from_millis(1000)).await;
                    let response_handle_mapping = self
                        .handle_mapping(hash, data_source, arc_templates.clone(), mapping, schema)
                        .await;
                    log::info!("Retry handle");
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn handle_mapping(
        &mut self,
        hash: &String,
        data_source: &DataSource,
        arc_templates: Arc<Vec<DataSourceTemplate>>,
        mapping: &PathBuf,
        schema: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        //sleep(Duration::from_millis(1000)).await;
        let chain_type = get_chain_type(data_source);
        let network = data_source.network.clone().unwrap_or(Default::default());
        let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone()).await?;
        let get_blocks_request = GetBlocksRequest {
            start_block_number: 0,
            end_block_number: 1,
            chain_type: chain_type as i32,
            network,
        };
        let mut stream: Streaming<GenericDataProto> = client
            .list_blocks(Request::new(get_blocks_request.clone()))
            .await?
            .into_inner();
        let mapping_response = match data_source.mapping.language.as_str() {
            "wasm/assemblyscript" => {
                self.handle_wasm_mapping(
                    hash,
                    data_source,
                    arc_templates.clone(),
                    schema,
                    &mut stream,
                )
                .await
            }
            //Default use rust
            _ => {
                self.handle_rust_mapping(hash, data_source, mapping, schema, &mut stream)
                    .await
            }
        };
        if mapping_response.is_ok() {
            log::info!("mapping_response Ok.");
        } else {
            log::error!("mapping_response Error: {:?}", &mapping_response);
        }
        mapping_response
    }

    async fn handle_wasm_mapping<P: AsRef<Path>>(
        &mut self,
        indexer_hash: &String,
        data_source: &DataSource,
        templates: Arc<Vec<DataSourceTemplate>>,
        schema_path: P,
        stream: &mut Streaming<GenericDataProto>,
    ) -> Result<(), Box<dyn Error>> {
        let store =
            Arc::new(StoreBuilder::create_store(indexer_hash.as_str(), &schema_path).unwrap());
        //let registry = Arc::new(MockMetricsRegistry::new());

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
            if let Some(ref mut proxy) = handler_proxy {
                match proxy.handle_wasm_mapping(&mut data) {
                    Err(err) => {
                        log::error!("{} Error while handle received message", err);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn handle_rust_mapping<P: AsRef<Path>>(
        &mut self,
        indexer_hash: &String,
        data_source: &DataSource,
        mapping_path: P,
        schema_path: P,
        stream: &mut Streaming<GenericDataProto>,
    ) -> Result<(), Box<dyn Error>> {
        let store = StoreBuilder::create_store(indexer_hash.as_str(), &schema_path).unwrap();
        let mut indexer_state = IndexerState::new(Arc::new(store));

        //Use unsafe to inject a store pointer into user's lib
        unsafe {
            match self
                .load(
                    indexer_hash,
                    mapping_path.as_ref().as_os_str(),
                    &indexer_state,
                )
                .await
            {
                Ok(_) => log::info!("{} Load library successfully", &*COMPONENT_NAME),
                Err(err) => log::error!("Load library with error {:?}", err),
            }
        }
        log::info!("{} Start mapping using rust", &*COMPONENT_NAME);
        let adapter_name = data_source
            .kind
            .split("/")
            .collect::<Vec<&str>>()
            .get(0)
            .unwrap()
            .to_string();
        if let Some(adapter_handler) = self.map_handlers.get_mut(indexer_hash.as_str()) {
            if let Some(handler_proxy) = adapter_handler.handler_proxies.get(&adapter_name) {
                while let Some(mut data) = stream.message().await? {
                    let start = Instant::now();
                    match handler_proxy.handle_rust_mapping(&mut data, &mut indexer_state) {
                        Err(err) => {
                            log::error!("{} Error while handle received message", err);
                        }
                        _ => {
                            //log::info!("Handler")
                        }
                    }
                    log::info!(
                        "{} Process chain {:?} with data block = {:?} hash = {:?}, data type = {:?} in {:?}",
                        &*COMPONENT_NAME,
                        ChainType::from_i32(data.chain_type).unwrap(),
                        data.block_number,
                        data.block_hash,
                        DataType::from_i32(data.data_type).unwrap(),
                        start.elapsed()
                    );
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
        store: &dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        let lib = Arc::new(Library::new(library_path)?);
        // inject store to plugin
        lib.get::<*mut Option<&dyn Store>>(b"STORE\0")?
            .write(Some(store));
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
    fn handle_rust_mapping(
        &self,
        message: &mut GenericDataProto,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn handle_wasm_mapping(
        &mut self,
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
