#[macro_use]
extern crate diesel;

use lazy_static::lazy_static;
use std::env;
use std::sync::Arc;

pub mod api;
pub mod indexer_service;

lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[IndexerApi]");
    pub static ref CONNECTION_POOL_SIZE: u32 = env::var("CONNECTION_POOL_SIZE")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(20);
    pub static ref API_ENDPOINT: String =
        env::var("API_ENDPOINT").unwrap_or(String::from("0.0.0.0:3031"));
    pub static ref DATABASE_URL: String = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://graph-node:let-me-in@localhost/analytic"
    ));
    pub static ref HASURA_URL: String =
        env::var("HASURA_URL").unwrap_or(String::from("http://127.0.0.1:8080/v1/query"));
    pub static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
    pub static ref GENERATED_FOLDER: String = String::from("index-manager/generated");
}
