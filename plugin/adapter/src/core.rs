use libloading::Library;
use serde_yaml::Value;
use tonic::Request;

use crate::setting::*;
pub use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
pub use crate::{handle_rust_mapping, HandlerProxyType, PluginRegistrar};
use index_store::core::{IndexStore, Store};
use lazy_static::lazy_static;
//use massbit_runtime_wasm::chain::ethereum::Chain;
//use massbit_runtime_wasm::indexer::manifest::{Mapping, MappingBlockHandler};
//use massbit_runtime_wasm::module::WasmInstance;

use massbit_chain_solana::data_type::{decode, SolanaEncodedBlock};
use std::{
    alloc::System, collections::HashMap, env, error::Error, ffi::OsStr, fmt, path::PathBuf,
    sync::Arc,
};

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
    /// Load a plugin library
    /// A plugin library **must** be implemented using the
    /// [`model::adapter_declaration!()`] macro. Trying manually implement
    /// a plugin without going through that macro will result in undefined
    /// behaviour.
    pub async unsafe fn load<P: AsRef<OsStr>>(
        &mut self,

        indexer_hash: String,
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
        self.libs.insert(indexer_hash, lib);
        Ok(())
    }

    pub async fn init(
        &mut self,
        hash: &String,
        config: &Value,
        mapping: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        let store = IndexStore::new(DATABASE_CONNECTION_STRING.as_str()).await;
        self.store = Some(store);
        unsafe {
            match self.load(hash.clone(), mapping).await {
                Ok(_) => {
                    log::info!("{} Load library successfully", &*COMPONENT_NAME)
                }
                Err(err) => {
                    println!("Load library with error {:?}", err)
                }
            }
        }
        let mut client = StreamoutClient::connect(CHAIN_READER_URL.clone()).await?;
        log::info!("{} Init Streamout client", &*COMPONENT_NAME);
        let chain_type = get_chain_type(&config);
        let _chain_name = get_chain_name(&config);
        let get_blocks_request = GetBlocksRequest {
            start_block_number: 0,
            end_block_number: 1,
            chain_type: chain_type as i32,
        };
        let mut stream = client
            .list_blocks(Request::new(get_blocks_request))
            .await?
            .into_inner();
        log::info!("{} Start processing block", &*COMPONENT_NAME);
        if let Some(adapter_handler) = self.map_handlers.get(hash) {
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
                let encoded_block: SolanaEncodedBlock = decode(&mut data.payload).unwrap();
                if let Some(adapter_name) = get_chain_name(&config) {
                    if let Some(handler_proxy) = adapter_handler.handler_proxies.get(adapter_name) {
                        match handle_rust_mapping(handler_proxy, &mut data) {
                            Err(err) => {
                                log::error!("{} Error while handle received message", err);
                            }
                            _ => {}
                        }
                    }
                } else {
                    log::warn!(
                        "{} Not support this chain-type {:?}",
                        &*COMPONENT_NAME,
                        chain_type
                    );
                }
            }
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
    /*
    fn handle_wasm_mapping(
        &self,
        wasm_instance: &mut WasmInstance<Chain>,
        mapping: &Mapping,
        message: &mut GenericDataProto,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
     */
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
