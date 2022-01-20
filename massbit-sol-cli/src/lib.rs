#[macro_use]
extern crate serde_derive;
pub mod config;
pub mod consts;
pub mod generator;
pub mod indexer_deploy;
pub mod indexer_release;
pub mod parser;
pub mod schema;

use lazy_static::lazy_static;
use std::env;
use std::path::PathBuf;
lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[SolanaCli]");
    pub static ref INDEXER_ENDPOINT: String =
        env::var("INDEXER_ENDPOINT").unwrap_or(String::from("http://127.0.0.1:3031"));
}
pub const METHOD_DEPLOY: &str = "indexer/deploy";
pub const INDEXER_LOGIC_FOLDER: &str = "indexer-logic";
pub const UNPACK_INSTRUCTION_FOLDER: &str = "unpack-instruction";
pub const SO_MAPPING_FILE_NAME: &str = "libblock.so";
pub const SO_UNPACK_INSTRUCTION_FILE_NAME: &str = "libunpack_instruction.so";
pub const SO_FOLDER: &str = "target/release";
pub const SCHEMA_FILE_NAME: &str = "schema.graphql";
pub const SUBGRAPH_FILE_NAME: &str = "subgraph.yaml";
pub const SRC_FOLDER: &str = "src";
pub const RELEASES_FOLDER: &str = "releases";
