/**
*** The objective of this file is to expose types / models
**/

// Generic dependencies
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Massbit dependencies
pub use stream_mod::{GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

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
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DetailParams {
    pub index_name: String,
    pub ipfs_config_hash: String,
    pub ipfs_mapping_hash: String,
}

pub struct IndexConfig {
    pub config: String,
    pub mapping: PathBuf,
    pub schema: String,
}