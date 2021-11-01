use crate::core::MessageHandler;
use index_store::Store;
use libloading::Library;
pub use massbit::firehose::bstream::BlockResponse;
use massbit_chain_solana::data_type::{decode, SolanaBlock, SolanaLogMessages, SolanaTransaction};
use paste::paste;

use std::{error::Error, sync::Arc};

crate::prepare_adapter!(Solana, {
     handle_block:SolanaBlock,
     handle_transaction:SolanaTransaction,
     handle_log_messages:SolanaLogMessages
});

impl MessageHandler for SolanaHandlerProxy {
    fn handle_rust_mapping(
        &self,
        data: &mut BlockResponse,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        let block: SolanaBlock = decode(&mut data.payload).unwrap();
        //let block = convert_solana_encoded_block_to_solana_block(encoded_block); // Decoding
        log::info!(
            "{} Received SOLANA BLOCK with block height: {:?}, hash: {:?}",
            &*COMPONENT_NAME,
            &block.block.block_height,
            &block.block.blockhash
        );
        self.handler.handle_block(&block);
        let mut print_flag = true;
        for origin_transaction in block.clone().block.transactions {
            let origin_log_messages = origin_transaction.meta.clone().unwrap().log_messages;
            let transaction = SolanaTransaction {
                block_number: ((&block).block.block_height.unwrap_or_default() as u32),
                transaction: origin_transaction.clone(),
                log_messages: origin_log_messages.clone(),
                success: false,
            };
            let log_messages = SolanaLogMessages {
                block_number: ((&block).block.block_height.unwrap_or_default() as u32),
                log_messages: origin_log_messages.clone(),
                transaction: origin_transaction.clone(),
            };
            if print_flag {
                log::info!(
                    "{} Recieved SOLANA TRANSACTION with Block number: {:?}, transaction: {:?}",
                    &*COMPONENT_NAME,
                    &transaction.block_number,
                    &transaction.transaction.transaction.signatures
                );
                log::info!(
                    "{} Recieved SOLANA LOG_MESSAGES with Block number: {:?}, log_messages: {:?}",
                    &*COMPONENT_NAME,
                    &log_messages.block_number,
                    &log_messages.log_messages.clone().unwrap().get(0)
                );
                print_flag = false;
            }
            self.handler.handle_transaction(&transaction);
            self.handler.handle_log_messages(&log_messages);
        }
        store.flush(&data.block_hash, data.block_number)
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
