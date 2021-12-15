#[macro_use]
extern crate diesel;
extern crate diesel_migrations;

pub mod indexer_service;
pub mod manager;
pub mod model;
pub mod orm;
pub mod server_builder;
pub mod store;

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::env;
use std::iter::FromIterator;

//Time out when get content from ipfs
pub const IPFS_TIME_OUT: u64 = 10_u64;
pub const API_LIST_LIMIT: i64 = 100_i64;
pub const GET_BLOCK_TIMEOUT_SEC: u64 = 600;
pub const GET_STREAM_TIMEOUT_SEC: u64 = 30;
pub const MAX_JSON_BODY_SIZE: u64 = 1024 * 1024;
lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[IndexerManager]");
    pub static ref CONNECTION_POOL_SIZE: u32 = env::var("CONNECTION_POOL_SIZE")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(20);
    pub static ref API_ENDPOINT: String =
        env::var("API_ENDPOINT").unwrap_or(String::from("0.0.0.0:3032"));
    pub static ref DATABASE_URL: String = env::var("DATABASE_URL")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    pub static ref CHAIN_READER_URL: String =
        env::var("CHAIN_READER_URL").unwrap_or(String::from("http://127.0.0.1:50051"));
    pub static ref HASURA_URL: String =
        env::var("HASURA_URL").unwrap_or(String::from("http://127.0.0.1:8080"));
    pub static ref IPFS_ADDRESS: String =
        env::var("IPFS_ADDRESS").unwrap_or(String::from("0.0.0.0:5001"));
    pub static ref GENERATED_FOLDER: String = String::from("indexer-manager/generated");
    pub static ref INDEXER_UPLOAD_DIR: String =
        env::var("INDEXER_UPLOAD_DIR").unwrap_or(String::from("."));
    pub static ref MAX_UPLOAD_FILE_SIZE: u64 = 10_000_000_u64;
    pub static ref FILES: HashMap<String, String> = HashMap::from_iter([
        (String::from("libblock.so"), String::from("mapping")),
        (String::from("schema.graphql"), String::from("schema")),
        (String::from("subgraph.yaml"), String::from("manifest")),
    ]);
}
