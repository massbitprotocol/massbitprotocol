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
    pub(crate) model_table_name: String,
    pub(crate) deploy_type: DeployType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DetailParams {
    pub(crate) index_name: String,
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
    pub index_data_ref: String,
}