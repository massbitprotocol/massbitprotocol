/**
 *** Objective of this file is to parse the config project.yaml file to get
 *** information like: chain type, index name, ...
 **/
// Generic dependencies
use serde_yaml::Value;
// Massbit dependencies
use graph_chain_ethereum::DataSource;
use massbit::firehose::bstream::ChainType;

//use massbit_runtime_wasm::chain::ethereum::data_source::DataSource;
pub fn get_chain_type(datasource: &DataSource) -> ChainType {
    let ds_kind = datasource.kind.split('/').next().unwrap();
    match ds_kind {
        "solana" => ChainType::Solana,
        "ethereum" => ChainType::Ethereum,
        _ => ChainType::Solana, // If not provided, assume it's Solana network
    }
}

pub fn get_chain_name(config: &Value) -> Option<&str> {
    config["dataSources"][0]["kind"].as_str()
}
pub fn get_index_name(config: &Value) -> String {
    let index_name = config["dataSources"][0]["name"].as_str().unwrap();
    String::from(index_name)
}
