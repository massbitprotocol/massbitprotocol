// Massbit dependencies
use ipfs_client::core::IpfsClient;
use serde::{Deserialize};

#[allow(dead_code)]
pub struct JsonRpcServer {
    http_addr: String,
    ipfs_client: Vec<IpfsClient>, // We need this to get user index config & mapping logic
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
}