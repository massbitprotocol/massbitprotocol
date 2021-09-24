#[macro_use]
extern crate paste;

pub mod core;
pub mod macros;
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
pub mod setting;
use crate::core::{AdapterHandler, MessageHandler};
use ethereum::EthereumHandlerProxy;
use quote::quote;
use std::{error::Error, sync::Arc};
use substrate::SubstrateHandlerProxy;
pub mod bsc;
pub mod ethereum;
//pub mod ipfs;
pub mod matic;
pub mod solana;
pub mod substrate;
use graph::blockchain::HostFn;
use graph::components::store::WritableStore;
use index_store::Store;
use std::collections::HashMap;
//Add new chain name in CamelCase here
// crate::create_adapters!(Matic, Bsc, Ethereum, Solana, Substrate);

use crate::bsc::*;
use crate::ethereum::*;
use crate::matic::*;
use crate::solana::*;
use crate::substrate::*;
pub enum HandlerProxyType {
    Matic(MaticHandlerProxy),
    Bsc(BscHandlerProxy),
    Ethereum(EthereumHandlerProxy),
    Solana(SolanaHandlerProxy),
    Substrate(SubstrateHandlerProxy),
    // $(
    //     $adapter([<$adapter HandlerProxy>])
    // ),*
}
impl MessageHandler for HandlerProxyType {
    fn handle_rust_mapping(
        &self,
        message: &mut GenericDataProto,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            HandlerProxyType::Matic(proxy) => proxy.handle_rust_mapping(message, store),
            HandlerProxyType::Bsc(proxy) => proxy.handle_rust_mapping(message, store),
            HandlerProxyType::Ethereum(proxy) => proxy.handle_rust_mapping(message, store),
            HandlerProxyType::Solana(proxy) => proxy.handle_rust_mapping(message, store),
            HandlerProxyType::Substrate(proxy) => proxy.handle_rust_mapping(message, store),
        }
    }
}
pub trait PluginRegistrar {
    fn register_matic_handler(&mut self, handler: Box<dyn MaticHandler + Send + Sync>);
    fn register_bsc_handler(&mut self, handler: Box<dyn BscHandler + Send + Sync>);
    fn register_ethereum_handler(&mut self, handler: Box<dyn EthereumHandler + Send + Sync>);
    fn register_solana_handler(&mut self, handler: Box<dyn SolanaHandler + Send + Sync>);
    fn register_substrate_handler(&mut self, handler: Box<dyn SubstrateHandler + Send + Sync>);
}
impl PluginRegistrar for AdapterHandler {
    fn register_matic_handler(&mut self, handler: Box<dyn MaticHandler + Send + Sync>) {
        self.handler_proxies.insert(
            format!("{}", quote!(Matic)),
            HandlerProxyType::Matic(MaticHandlerProxy::new(handler, Arc::clone(&self.lib))),
        );
    }
    fn register_bsc_handler(&mut self, handler: Box<dyn BscHandler + Send + Sync>) {
        self.handler_proxies.insert(
            format!("{}", quote!(Bsc)),
            HandlerProxyType::Bsc(BscHandlerProxy::new(handler, Arc::clone(&self.lib))),
        );
    }
    fn register_ethereum_handler(&mut self, handler: Box<dyn EthereumHandler + Send + Sync>) {
        self.handler_proxies.insert(
            format!("{}", quote!(Ethereum)),
            HandlerProxyType::Ethereum(EthereumHandlerProxy::new(handler, Arc::clone(&self.lib))),
        );
    }
    fn register_solana_handler(&mut self, handler: Box<dyn SolanaHandler + Send + Sync>) {
        self.handler_proxies.insert(
            format!("{}", quote!(Solana)),
            HandlerProxyType::Solana(SolanaHandlerProxy::new(handler, Arc::clone(&self.lib))),
        );
    }
    fn register_substrate_handler(&mut self, handler: Box<dyn SubstrateHandler + Send + Sync>) {
        self.handler_proxies.insert(
            format!("{}", quote!(Substrate)),
            HandlerProxyType::Substrate(SubstrateHandlerProxy::new(handler, Arc::clone(&self.lib))),
        );
    }
}

// pub trait PluginRegistrar {

//     fn [<register_ $adapter:lower _handler>](&mut self, handler: Box<dyn [<$adapter Handler>] + Send + Sync>);

// }

// impl PluginRegistrar for AdapterHandler {
//     $(
//     fn [<register_ $adapter:lower _handler>](&mut self, handler: Box<dyn [<$adapter Handler>] + Send + Sync>) {
//         self.handler_proxies.insert(
//                 format!("{}", quote!([<$adapter:lower>])),
//                 HandlerProxyType::$adapter([<$adapter HandlerProxy>]::new(handler, Arc::clone(&self.lib))));

//     }
//     )*
// }

// crate::create_wasm_adapters!(Ethereum);

//use massbit_runtime_wasm::mapping::ValidModule;
use graph_chain_ethereum::{DataSource, DataSourceTemplate};
//use massbit_runtime_wasm::mapping::MappingContext;
use graph_runtime_wasm::ValidModule;
//use graph_runtime_wasm::{ValidModule, MappingContext, WasmInstance};

pub struct EthereumWasmHandlerProxy {
    pub indexer_hash: String,
    pub store: Arc<dyn WritableStore>,
    pub data_sources: Vec<DataSource>,
    pub templates: Arc<Vec<DataSourceTemplate>>,
    pub wasm_modules: HashMap<String, Arc<ValidModule>>,
    pub ethereum_calls: HashMap<String, HostFn>, //pub registry: Arc<dyn MetricsRegistry>
}
impl EthereumWasmHandlerProxy {
    pub fn new(
        indexer_hash: &String,
        store: Arc<dyn WritableStore>,
        data_source: DataSource,
        templates: Arc<Vec<DataSourceTemplate>>,
    ) -> EthereumWasmHandlerProxy {
        EthereumWasmHandlerProxy {
            indexer_hash: indexer_hash.clone(),
            store,
            data_sources: vec![data_source],
            templates,
            wasm_modules: HashMap::default(),
            ethereum_calls: HashMap::default(), //registry: Arc::new(MockMetricsRegistry::new())
        }
    }
    pub fn add_data_source(&mut self, datasource: DataSource) {
        self.data_sources.push(datasource);
    }
}
/*
impl EthereumWasmHandlerProxy {
    pub fn new(wasm_module: Arc<ValidModule>) -> EthereumWasmHandlerProxy {
        EthereumWasmHandlerProxy {
            wasm_module
        }
    }
}
 */

pub enum WasmHandlerProxyType {
    Ethereum(EthereumWasmHandlerProxy),
}
impl WasmHandlerProxyType {
    pub fn create_proxy(
        adapter_name: &String,
        indexer_hash: &String,
        store: Arc<dyn WritableStore>,
        data_source: DataSource,
        templates: Arc<Vec<DataSourceTemplate>>,
    ) -> Option<WasmHandlerProxyType> {
        log::info!("Create proxy for adapter {}", adapter_name);
        let mut proxy = None;

        if format!("{}", quote!(ethereum)).eq(adapter_name) {
            proxy = Some(WasmHandlerProxyType::Ethereum(
                EthereumWasmHandlerProxy::new(indexer_hash, store, data_source, templates),
            ));
        }

        proxy
    }
}
/*
impl WasmHandlerProxyType {
    pub fn create_proxy(adapter_name: &String, wasm_module : Arc<ValidModule>) -> Option<WasmHandlerProxyType> {
        log::info!("Create proxy for adapter {}", adapter_name);
        let mut proxy = None;
            $(
            if format!("{}", quote!([<$adapter:lower>])).eq(adapter_name) {
                proxy = Some(WasmHandlerProxyType::$adapter(EthereumWasmHandlerProxy::new(Arc::clone(&wasm_module))));
            }
            )*

        proxy
    }
}
*/
impl MessageHandler for WasmHandlerProxyType {
    fn handle_wasm_mapping(
        &mut self,
        //wasm_instance: &mut WasmInstance<Chain>,
        //datasource: &DataSource,
        message: &mut GenericDataProto,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            WasmHandlerProxyType::Ethereum(proxy) => {
                //proxy.handle_wasm_mapping(wasm_instance, datasource, message)
                proxy.handle_wasm_mapping(message)
            }
        }
    }
}
/*
pub fn handle_wasm_mapping(
        proxy_type: &WasmHandlerProxyType,
        wasm_instance: &mut WasmInstance<Chain>,
        mapping: &Mapping,
        message: &mut GenericDataProto
) -> Result<(), Box<dyn Error>> {
    match proxy_type {
        $(
        WasmHandlerProxyType::$adapter(proxy) => {
                proxy.handle_wasm_mapping(wasm_instance, mapping, message)
            }
        )*
    }
}
 */

#[macro_export]
macro_rules! export_plugin {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static adapter_declaration: $crate::core::AdapterDeclaration =
            $crate::core::AdapterDeclaration {
                register: $register,
            };
    };
}
