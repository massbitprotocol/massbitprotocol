/**
*** Objective of this file is to parse the config project.yaml file to get
*** information like: chain type, index name, ...
**/

// Generic dependencies
use serde_yaml::Value;
use std::iter;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
// Massbit dependencies
use crate::types::stream_mod::{ChainType};

pub fn get_chain_type(config: &Value) -> ChainType {
    let chain_type = match config["dataSources"][0]["kind"].as_str().unwrap() {
        "substrate" => ChainType::Substrate,
        "solana" => ChainType::Solana,
        _ => ChainType::Substrate, // If not provided, assume it's substrate network
    };
    chain_type
}

pub fn get_index_name(config: &Value) -> String {
    let index_name = config["dataSources"][0]["name"].as_str().unwrap();
    String::from(index_name)
}

// Random hash for every new index so it will be unique
pub fn generate_random_hash() -> String {
    let mut rng = thread_rng();
    let chars: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(20)
        .collect();
    chars
}