use crate::plugin::handler::SolanaHandler;
use crate::types::{SolanaBlock, SolanaTransaction};
use std::error::Error;
use std::sync::Mutex;

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
        println!(
            "Start handle_blocks SolanaHandlerProxy, block len: {}",
            blocks.len()
        );
        self.handler.handle_blocks(blocks)
    }
}
