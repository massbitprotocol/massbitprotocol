/**
*** The objective of this file is to expose types / models
**/
// Generic dependencies
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum_macros::AsStaticStr;

// Massbit dependencies
pub use stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

// The order of params is important to correctly map the API request to this struct
#[derive(Clone, Debug, Deserialize)]
pub struct DeployParams {
    pub config: String,
    pub mapping: String,
    pub schema: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Indexer {
    pub id: String,
    pub network: String,
    pub name: String,
    pub hash: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DetailParams {
    pub index_name: String,
    pub ipfs_config_hash: String,
    pub ipfs_mapping_hash: String,
}

// The IndexConfig is built from DeployParams
// IndexConfig should let us know the location where the configs are stored
pub struct IndexConfig {
    pub config: PathBuf,
    pub mapping: PathBuf,
    pub schema: PathBuf,
    pub identifier: IndexIdentifier,
}

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
