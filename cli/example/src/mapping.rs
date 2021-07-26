use crate::models::*;
use massbit_chain_substrate::data_type as types;
use std::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub fn handle_block(block: &types::SubstrateBlock) -> Result<(), Box<dyn Error>> {
    unimplemented!();
}

#[derive(Debug, Clone, PartialEq)]
pub fn handle_extrinsic(extrinsic: &types::SubstrateExtrinsic) -> Result<(), Box<dyn Error>> {
    todo!();
}

#[derive(Debug, Clone, PartialEq)]
pub fn handle_event(event: &types::SubstrateEventRecord) -> Result<(), Box<dyn Error>> {
    todo!();
}

#[derive(Debug, Clone, PartialEq)]
pub fn handle_block(block: &types::SolanaBlock) -> Result<(), Box<dyn Error>> {
    todo!();
}

#[derive(Debug, Clone, PartialEq)]
pub fn handle_transaction(transaction: &types::SolanaTransaction) -> Result<(), Box<dyn Error>> {
    unimplemented!();
}