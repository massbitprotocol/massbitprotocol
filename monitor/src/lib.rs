pub mod opt;
pub mod service;
use massbit_common::prelude::lazy_static::lazy_static;
use std::env;

lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[IndexerMonitor]");
    pub static ref CONNECTION_POOL_SIZE: u32 = env::var("CONNECTION_POOL_SIZE")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(20);
    pub static ref API_ENDPOINT: String =
        env::var("API_ENDPOINT").unwrap_or(String::from("0.0.0.0:8080"));
    pub static ref DATABASE_URL: String = env::var("DATABASE_URL").unwrap_or(String::from(
        "postgres://graph-node:let-me-in@localhost/graph-node"
    ));
    pub static ref MONITOR_PERIOD: u64 = std::env::var("MONITOR_PERIOD")
        .ok()
        .and_then(|val| val.parse().ok())
        .unwrap_or(10000);
}
