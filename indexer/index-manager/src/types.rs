// Massbit dependencies
use serde::{Deserialize};

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
    pub(crate) deploy_type: DeployType,
}

#[derive(Clone, Debug, Deserialize)]
pub enum DeployType {
    Local,
    Ipfs,
}