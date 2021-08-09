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

//Add new chain name in CamelCase here
crate::create_adapters!(Matic, Bsc, Ethereum, Solana, Substrate);

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
