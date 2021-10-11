/**
*** The objective of this file is to expose types / models
**/
// Generic dependencies
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum_macros::AsStaticStr;

// Massbit dependencies
// pub use stream_mod::{
//     streamout_client::StreamoutClient, ChainType, DataType, BlockResponse, GetBlocksRequest,
// };
// pub mod stream_mod {
//     tonic::include_proto!("chaindata");
// }

// Indexer details that are extract right from the Database
#[derive(Deserialize, Serialize, Debug)]
pub struct Indexer {
    pub id: String,
    pub network: String,
    pub name: String,
    pub hash: String,
}

// Normalized version of DeployAbi
#[derive(Clone, Debug, Deserialize)]
pub struct Abi {
    pub name: String,
    pub path: PathBuf,
}

// The IndexConfig is built from DeployParams
// IndexConfig should let us know the location where the configs are
#[derive(Clone, Debug)]
pub struct IndexConfig {
    pub config: PathBuf,
    pub mapping: PathBuf,
    pub schema: PathBuf,
    pub abi: Option<Vec<Abi>>, // .SO doesn't support uploading ABIs yet, only .WASM need the ABIs
    pub identifier: IndexIdentifier,
    pub subgraph: PathBuf,
}

// Identifier of the IndexConfig is an helper to easily access the hash of the index, and index's file name
#[derive(Clone, Debug)]
pub struct IndexIdentifier {
    pub name: String,
    pub hash: String,
    pub name_with_hash: String,
}

// This is inspired by the syncing status from eth https://ethereum.stackexchange.com/questions/69458/sync-status-of-ethereum-node
#[derive(Clone, Debug, PartialEq, AsStaticStr)]
pub enum IndexStatus {
    Synced,  // Meaning that the index is running
    Syncing, // This mean our index is not caught up to the latest block yet. We don't support this field yet
    False,   // Meaning that the index is not running
}

pub struct IndexStore {}
