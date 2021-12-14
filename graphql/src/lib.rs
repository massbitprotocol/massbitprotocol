pub mod config;
pub mod execution;
pub mod introspection;
pub mod opt;
pub mod query;
pub mod runner;
pub mod server;
pub mod store;
pub mod store_builder;
pub mod values;

use massbit_common::prelude::lazy_static::lazy_static;
use std::env;

lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[GraphqlApi]");
    pub static ref CONNECTION_POOL_SIZE: u32 = env::var("CONNECTION_POOL_SIZE")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(20);
    pub static ref API_ENDPOINT: String =
        env::var("API_ENDPOINT").unwrap_or(String::from("0.0.0.0:8080"));
    pub static ref DATABASE_URL: String = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://graph-node:let-me-in@localhost/graph-node"
    ));
    pub static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));

    // graph_node::config disallows setting this in a store with multiple
    // shards. See 8b6ad0c64e244023ac20ced7897fe666 for the reason
    pub static ref CLEANUP_BLOCKS: bool = std::env::var("CLEANUP_BLOCKS")
        .ok()
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
}
