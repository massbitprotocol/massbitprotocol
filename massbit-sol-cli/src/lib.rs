#[macro_use]
extern crate serde_derive;
pub mod generator;
pub mod indexer_deploy;
pub mod indexer_release;
pub mod parser;
pub mod schema;

use lazy_static::lazy_static;
use std::env;
lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[SolanaCli]");
    pub static ref INDEXER_ENDPOINT: String =
        env::var("INDEXER_ENDPOINT").unwrap_or(String::from("http://127.0.0.1:3031"));
}
pub const METHOD_DEPLOY: &str = "indexer/deploy";
