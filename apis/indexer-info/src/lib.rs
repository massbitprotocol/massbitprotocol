#[macro_use]
extern crate diesel;
extern crate diesel_migrations;

// pub mod git_helper;
pub mod indexer_service;
// pub mod manager;
pub mod model;
pub mod orm;
pub mod server_builder;
// pub mod store;

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::env;
use std::iter::FromIterator;

//Time out when get content from ipfs
pub const API_LIST_LIMIT: i64 = 100_i64;
pub const MAX_JSON_BODY_SIZE: u64 = 1024 * 1024;
lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[IndexerInfo]");
    pub static ref CONNECTION_POOL_SIZE: u32 = env::var("CONNECTION_POOL_SIZE")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(20);
    pub static ref INFO_ENDPOINT: String =
        env::var("INFO_ENDPOINT").unwrap_or(String::from("0.0.0.0:3031"));
    pub static ref API_ENDPOINT: String =
        env::var("API_ENDPOINT").unwrap_or(String::from("http://localhost:3032"));
    pub static ref API_DEPLOY_ENDPOINT: String = API_ENDPOINT.clone() + "/indexers/gitdeploy";
    pub static ref DATABASE_URL: String = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://graph-node:let-me-in@localhost/graph-node"
    ));
}
