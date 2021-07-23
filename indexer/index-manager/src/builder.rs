use std::path::PathBuf;

// Massbit dependencies
use crate::helper::{get_mapping_file_from_ipfs, get_config_file_from_ipfs, get_raw_query_from_local, get_config_file_from_local, get_mapping_file_from_local};
use crate::types::{DeployType, IndexConfig};
use crate::config_helper::get_raw_query_from_ipfs;

/**
*** Builder Pattern
*** Real example: https://github.com/graphprotocol/rust-web3/blob/3aac17f719b99494793111fd00a4505fe4670ca2/src/types/log.rs#L103
*** Advantages:
***  - Separates methods for building from other methods.
***  - Prevents proliferation of constructors
***  - Can be used for one-liner initialisation as well as more complex construction.
*** Note:
***  - I think this is useful when there's too many complex check that needs to be done and we want to hide it from the main logic
*** Reference: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
**/

/*********************
* Index Config Local *
*********************/
pub struct IndexConfigLocalBuilder {
    model: String,
    config: String,
    mapping: PathBuf,
    query: String,
}

impl Default for IndexConfigLocalBuilder {
    fn default() -> IndexConfigLocalBuilder {
        IndexConfigLocalBuilder {
            model: "".to_string(),
            config: "".to_string(),
            mapping: "".to_string().parse().unwrap(),
            query: "".to_string(),
        }
    }
}

impl IndexConfigLocalBuilder {
    fn model(mut self, model: String) -> IndexConfigLocalBuilder {
        self.model = model;
        self
    }

    pub fn query(mut self, query: String) -> IndexConfigLocalBuilder {
        self.query = get_raw_query_from_local(&query);
        self
    }

    pub fn mapping(mut self, mapping: String) -> IndexConfigLocalBuilder {
        self.mapping = get_mapping_file_from_local(&mapping);
        self
    }

    pub fn config(mut self, config: String) -> IndexConfigLocalBuilder {
        self.config = get_config_file_from_local(&config);
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            model: self.model,
            config: self.config,
            mapping: self.mapping,
            query: self.query,
        }
    }
}

/********************
* Index Config IPFS *
********************/
pub struct IndexConfigIpfsBuilder {
    model: String,
    config: String,
    mapping: PathBuf,
    query: String,
}

impl Default for IndexConfigIpfsBuilder {
    fn default() -> IndexConfigIpfsBuilder {
        IndexConfigIpfsBuilder {
            model: "".to_string(),
            config: "".to_string(),
            mapping: "".to_string().parse().unwrap(),
            query: "".to_string(),
        }
    }
}

impl IndexConfigIpfsBuilder {
    fn model(mut self, model: String) -> IndexConfigIpfsBuilder {
        self.model = model;
        self
    }

    pub async fn query(mut self, query: String) -> IndexConfigIpfsBuilder {
        self.query = get_raw_query_from_ipfs(&query).await;
        self
    }

    pub async fn mapping(mut self, mapping: String) -> IndexConfigIpfsBuilder {
        let mapping_file_name = get_mapping_file_from_ipfs(&mapping).await;
        let mapping_file_location = ["./", &mapping_file_name].join("");
        self.mapping = PathBuf::from(mapping_file_location.to_string());
        self
    }

    pub async fn config(mut self, config: String) -> IndexConfigIpfsBuilder {
        self.config = get_config_file_from_ipfs(&config).await;
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            model: self.model,
            config: self.config,
            mapping: self.mapping,
            query: self.query,
        }
    }
}

