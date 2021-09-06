extern crate diesel;
use graph::data::subgraph::DeploymentHash;
use lazy_static::lazy_static;
pub mod core;
//pub mod mapping;
pub mod postgres;

lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[Index-Store]");
    pub static ref DEPLOYMENT_HASH: DeploymentHash = DeploymentHash::new("_indexer").unwrap();
}

pub use crate::core::Store;
//pub use crate::mapping::IndexerState;
