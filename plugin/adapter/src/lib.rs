#[macro_use]
extern crate paste;

pub mod core;
pub mod macros;
pub mod setting;
use crate::core::{AdapterHandler, MessageHandler};
use quote::quote;
use std::{error::Error, sync::Arc};
pub mod solana;
use graph::blockchain::HostFn;
use graph::components::store::WritableStore;
use index_store::Store;
use std::collections::HashMap;

crate::create_adapters!(Solana);

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
