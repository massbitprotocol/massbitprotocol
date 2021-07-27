/**
*** Objective of this file, is to build the IndexConfig from the user's Index Request
*** It will connect to IPFS to get the files and save them to storage
**/

// Generic dependencies
use std::path::PathBuf;
// Massbit dependencies
use crate::types::{IndexConfig};
use crate::ipfs::{get_mapping_ipfs, get_config_ipfs, get_schema_ipfs};

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
pub struct IndexConfigIpfsBuilder {
    schema: String,
    config: String,
    mapping: PathBuf,
}

impl Default for IndexConfigIpfsBuilder {
    fn default() -> IndexConfigIpfsBuilder {
        IndexConfigIpfsBuilder {
            schema: Default::default(),
            config: Default::default(),
            mapping: "".to_string().parse().unwrap(),
        }
    }
}

impl IndexConfigIpfsBuilder {
    pub async fn mapping(mut self, mapping: String) -> IndexConfigIpfsBuilder {
        let mapping_name = get_mapping_ipfs(&mapping).await;
        let mapping_file = ["./", &mapping_name].join("");
        self.mapping = PathBuf::from(mapping_file.to_string());
        self
    }

    pub async fn config(mut self, config: String) -> IndexConfigIpfsBuilder {
        self.config = get_config_ipfs(&config).await;
        self
    }

    pub async fn schema(mut self, schema: String) -> IndexConfigIpfsBuilder {
        self.schema = get_schema_ipfs(&schema).await;
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            schema: self.schema,
            config: self.config,
            mapping: self.mapping,
        }
    }
}

