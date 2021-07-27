use serde::{Deserialize, Serialize};
use std::path::PathBuf;
pub use stream_mod::{GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};
pub mod stream_mod {
    tonic::include_proto!("chaindata");
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeployParams {
    pub index_name: String,
    pub config: String,
    pub mapping: String,
    pub query: String,
    pub table_name: String,
    pub deploy_type: DeployType,
    pub schema: String,
}

#[derive(Clone, Debug, Deserialize)]
pub enum DeployType {
    Local,
    Ipfs,
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
    pub query: String,
    pub schema: String,
}