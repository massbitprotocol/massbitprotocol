use serde_yaml::Value;
use crate::types::stream_mod::{GetBlocksRequest, GenericDataProto, ChainType, DataType, streamout_client::StreamoutClient};

pub fn get_chain_type(config: &Value) -> ChainType {
    let chain_type = match config["dataSources"][0]["kind"].as_str().unwrap() {
        "substrate" => ChainType::Substrate,
        "solana" => ChainType::Solana,
        _ => ChainType::Substrate, // If not provided, assume it's substrate network
    };
    chain_type
}