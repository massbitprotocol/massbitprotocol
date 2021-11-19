pub mod handler;

pub use handler::SolanaHandlerProxy;
use index_store::Store;
use libloading::Library;
pub use massbit::firehose::bstream::BlockResponse;
use massbit_chain_solana::data_type::{decode, SolanaBlock, SolanaLogMessages, SolanaTransaction};
use std::{error::Error, sync::Arc};

// crate::prepare_adapter!(Solana, {
//      handle_block:SolanaBlock,
//      handle_transaction:SolanaTransaction,
//      handle_log_messages:SolanaLogMessages
// });

pub trait SolanaHandler {
    fn handle_block(&self, _message: &SolanaBlock) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn handle_transaction(&self, _message: &SolanaTransaction) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn handle_log_messages(&self, _message: &SolanaLogMessages) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

/*
create_adapter!(SolanaHandler, SolanaHandlerProxy, SolanaRegistrarTrait, register_solana_handler, {
     handle_block:SolanaBlock,
     handle_transaction:SolanaTransaction,
     handle_log_messages:SolanaLogMessages
});
 */
/*
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(SolanaRegistrar)]
pub fn registrar_adapter(attr: TokenStream,input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    let output = quote! {
    impl SolanaRegistrarTrait for #ident {
        fn register_solana_handler(&mut self, handler: Box<dyn SolanaHandler>) {
            let proxy = SolanaHandlerProxy::new(handler, Rc::clone(&self.lib));
            self.solana_handler_proxies.insert(self.adapter_id.clone(), proxy);
        }
    }
    };
    output.into()
}
*/
