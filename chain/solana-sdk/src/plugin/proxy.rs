use crate::plugin::handler::SolanaHandler;
use crate::store::IndexStore;
use crate::types::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use crate::COMPONENT_NAME;
use std::error::Error;
use std::sync::{Arc, Mutex};

/// A proxy object which wraps a [`Handler`] and makes sure it can't outlive
/// the library it came from.
pub struct SolanaHandlerProxy {
    pub handler: Box<dyn SolanaHandler + Send + Sync>,
}
impl SolanaHandlerProxy {
    pub fn new(handler: Box<dyn SolanaHandler + Send + Sync>) -> SolanaHandlerProxy {
        SolanaHandlerProxy { handler }
    }
}
impl SolanaHandler for SolanaHandlerProxy {
    fn handle_blocks(&self, blocks: &Vec<SolanaBlock>) -> Result<i64, Box<dyn Error>> {
        self.handler.handle_blocks(blocks)
    }
}
