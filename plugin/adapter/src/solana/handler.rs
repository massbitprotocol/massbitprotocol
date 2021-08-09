use crate::core::{AdapterManager, AdapterRegistrar};
use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use std::{alloc::System, collections::HashMap, error::Error, ffi::OsStr, rc::Rc};
use libloading::Library;

/*
pub trait SolanaHandler {
    fn handle_block(&mut self, block: &SolanaBlock) -> Result<(), Box<dyn Error>>;
    fn handle_transaction(&self, extrinsic: &SolanaTransaction) -> Result<(), Box<dyn Error>>;
    fn handle_log_messages(&self, event: &SolanaLogMessages) -> Result<(), Box<dyn Error>>;
}
/*
/// A proxy object which wraps a [`Handler`] and makes sure it can't outlive
/// the library it came from.
pub struct SolanaHandlerProxy {
    handler: Arc<dyn SolanaHandler>,
    _lib: Rc<Library>,
}
impl SolanaHandlerProxy {
    pub fn new(handler: Box<dyn SolanaHandler>, _lib: Rc<Library>) -> SolanaHandlerProxy {
        SolanaHandlerProxy {
            handler,
            _lib
        }
    }
    pub fn get_handler(&self) -> &Box<dyn SolanaHandler> {
        &self.handler
    }
}
impl SolanaHandler for SolanaHandlerProxy {
    fn handle_block(&mut self, block: &SolanaBlock) -> Result<(), Box<dyn Error>> {
    fn handle_transaction(&self, extrinsic: &SolanaTransaction) -> Result<(), Box<dyn Error>>;
    fn handle_log_messages(&self, event: &SolanaLogMessages) -> Result<(), Box<dyn Error>>;
}
*/
