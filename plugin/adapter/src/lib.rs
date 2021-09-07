#[macro_use]
extern crate paste;

pub mod core;
pub mod macros;
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}
pub mod setting;
use crate::core::{AdapterHandler, MessageHandler};
use quote::quote;
use std::{error::Error, sync::Arc};
pub mod bsc;
pub mod ethereum;
//pub mod ipfs;
pub mod matic;
pub mod solana;
pub mod substrate;
use graph::blockchain::HostFn;
use graph::components::store::{ModificationsAndCache, StoreError, WritableStore};
use graph::prelude::MetricsRegistry;
use graph_mock::MockMetricsRegistry;
use std::collections::HashMap;
//Add new chain name in CamelCase here
crate::create_adapters!(Matic, Bsc, Ethereum, Solana, Substrate);
crate::create_wasm_adapters!(Ethereum);
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
