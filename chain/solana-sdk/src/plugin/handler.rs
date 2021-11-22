use crate::types::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use std::error::Error;

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
