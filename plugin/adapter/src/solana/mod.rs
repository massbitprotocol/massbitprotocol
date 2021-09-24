use crate::core::{AdapterError, MessageHandler};
pub use crate::stream_mod::{DataType, GenericDataProto};
use index_store::Store;
use libloading::Library;
use massbit_chain_solana::data_type::{
    convert_solana_encoded_block_to_solana_block, decode, SolanaBlock, SolanaEncodedBlock,
    SolanaLogMessages, SolanaTransaction,
};
use paste::paste;
use std::result::Result::Err;
use std::{error::Error, sync::Arc};

// crate::prepare_adapter!(Solana, {
//      handle_block:SolanaBlock,
//      handle_transaction:SolanaTransaction,
//      handle_log_messages:SolanaLogMessages
// });

lazy_static::lazy_static! {
    static ref COMPONENT_NAME: String = String::from(format!("[{}-Adapter]", quote::quote!(Solana)));
}
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
/// A proxy object which wraps a [`Handler`] and makes sure it can't outlive
/// the library it came from.
pub struct SolanaHandlerProxy {
    pub handler: Box<dyn SolanaHandler + Send + Sync>,
    _lib: Arc<Library>,
}
impl SolanaHandlerProxy {
    pub fn new(
        handler: Box<dyn SolanaHandler + Send + Sync>,
        _lib: Arc<Library>,
    ) -> SolanaHandlerProxy {
        SolanaHandlerProxy { handler, _lib }
    }
}
impl SolanaHandler for SolanaHandlerProxy {
    fn handle_block(&self, message: &SolanaBlock) -> Result<(), Box<dyn Error>> {
        self.handler.handle_block(message)
    }

    fn handle_transaction(&self, message: &SolanaTransaction) -> Result<(), Box<dyn Error>> {
        self.handler.handle_transaction(message)
    }

    fn handle_log_messages(&self, message: &SolanaLogMessages) -> Result<(), Box<dyn Error>> {
        self.handler.handle_log_messages(message)
    }
}

impl MessageHandler for SolanaHandlerProxy {
    fn handle_rust_mapping(
        &self,
        data: &mut GenericDataProto,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        /*
        println!(
            "{} handle rust mapping with payload {:?}",
            &*COMPONENT_NAME, &data.payload
        );
         */
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let encoded_block: SolanaEncodedBlock = decode(&mut data.payload).unwrap();
                let block = convert_solana_encoded_block_to_solana_block(encoded_block); // Decoding
                log::info!(
                    "{} Received SOLANA BLOCK with block height: {:?}, hash: {:?}",
                    &*COMPONENT_NAME,
                    &block.block.block_height.unwrap(),
                    &block.block.blockhash
                );
                self.handler.handle_block(&block);
                let mut print_flag = true;
                for origin_transaction in block.clone().block.transactions {
                    let origin_log_messages = origin_transaction.meta.clone().unwrap().log_messages;
                    let transaction = SolanaTransaction {
                        block_number: ((&block).block.block_height.unwrap() as u32),
                        transaction: origin_transaction.clone(),
                        log_messages: origin_log_messages.clone(),
                        success: false,
                    };
                    let log_messages = SolanaLogMessages {
                        block_number: ((&block).block.block_height.unwrap() as u32),
                        log_messages: origin_log_messages.clone(),
                        transaction: origin_transaction.clone(),
                    };
                    if print_flag {
                        log::info!("{} Recieved SOLANA TRANSACTION with Block number: {:?}, transaction: {:?}", &*COMPONENT_NAME, &transaction.block_number, &transaction.transaction.transaction.signatures);
                        log::info!("{} Recieved SOLANA LOG_MESSAGES with Block number: {:?}, log_messages: {:?}", &*COMPONENT_NAME, &log_messages.block_number, &log_messages.log_messages.clone().unwrap().get(0));
                        print_flag = false;
                    }
                    self.handler.handle_transaction(&transaction);
                    self.handler.handle_log_messages(&log_messages);
                }
                store.flush(&data.block_hash, data.block_number)
            }
            _ => {
                log::warn!(
                    "{} Not support data type: {:?}",
                    &*COMPONENT_NAME,
                    &data.data_type
                );
                Err(Box::new(AdapterError::new(
                    format!("Not support data type: {:?}", &data.data_type).as_str(),
                )))
            }
        }
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
