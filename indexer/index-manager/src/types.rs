use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use serde_yaml::Value;

#[allow(dead_code)]
pub struct IndexManager {
    http_addr: String,
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
    pub config: Value,
    pub mapping: PathBuf,
    pub query: String,
}