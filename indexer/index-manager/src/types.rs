use serde::{Deserialize, Serialize};

#[allow(dead_code)]
pub struct IndexManager {
    http_addr: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeployParams {
    pub(crate) index_name: String,
    pub(crate) config_path: String,
    pub(crate) mapping_path: String,
    pub(crate) model_path: String,
    pub(crate) table_name: String,
    pub(crate) deploy_type: DeployType,
    pub(crate) schema: String,
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
    pub(crate) index_name: String,
    pub(crate) ipfs_config_hash: String,
    pub(crate) ipfs_mapping_hash: String,
}