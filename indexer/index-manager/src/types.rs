// Massbit dependencies
use serde::{Deserialize};

#[allow(dead_code)]
pub struct IndexManager {
    http_addr: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeployLocalParams {
    index_name: String,
    config_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeployIpfsParams {
    pub(crate) index_name: String,
    pub(crate) ipfs_config_hash: String,
    pub(crate) ipfs_mapping_hash: String,
    pub(crate) ipfs_model_hash: String,
}