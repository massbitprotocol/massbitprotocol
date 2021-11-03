#[macro_use]
extern crate paste;

pub mod core;
//pub mod macros;
pub mod setting;
use crate::core::{AdapterHandler, MessageHandler};
use quote::quote;
use std::{error::Error, sync::Arc};
pub mod solana;

use crate::solana::SolanaHandlerProxy;
use index_store::Store;
use massbit::firehose::bstream::SolanaTransactionsResponse;

//crate::create_adapters!(Solana);

pub enum HandlerProxyType {
    Solana(SolanaHandlerProxy),
}
impl MessageHandler for HandlerProxyType {
    fn handle_transaction_mapping(
        &self,
        message: &mut SolanaTransactionsResponse,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            HandlerProxyType::Solana(proxy) => proxy.handle_transaction_mapping(message, store),
        }
    }
}
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
