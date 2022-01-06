use crate::manager::buffer::IncomingBlocks;
use std::error::Error;
use std::sync::Arc;

pub struct BlockStream {
    address: String,
    buffer: Arc<IncomingBlocks>,
}

impl BlockStream {
    pub fn new(address: String, buffer: Arc<IncomingBlocks>) -> Self {
        Self { address, buffer }
    }
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        loop {}
        Ok(())
    }
}
