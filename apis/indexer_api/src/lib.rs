#[macro_use]
extern crate diesel;
extern crate diesel_migrations;
use lazy_static::lazy_static;
use std::env;

pub mod indexer_service;
pub mod model;
pub mod orm;
pub mod server_builder;

//Time out when get content from ipfs
pub const IPFS_TIME_OUT: u64 = 10_u64;
lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[IndexerApi]");
    pub static ref CONNECTION_POOL_SIZE: u32 = env::var("CONNECTION_POOL_SIZE")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(20);
    pub static ref API_ENDPOINT: String =
        env::var("API_ENDPOINT").unwrap_or(String::from("0.0.0.0:3031"));
    pub static ref DATABASE_URL: String = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://graph-node:let-me-in@localhost/graph-node"
    ));
    pub static ref HASURA_URL: String =
        env::var("HASURA_URL").unwrap_or(String::from("http://127.0.0.1:8080/v1/query"));
    pub static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
    pub static ref GENERATED_FOLDER: String = String::from("index-manager/generated");
    pub static ref INDEXER_UPLOAD_DIR: String =
        env::var("INDEXER_UPLOAD_DIR").unwrap_or(String::from("."));
    pub static ref MAX_UPLOAD_FILE_SIZE: u64 = 10_000_000_u64;
}
