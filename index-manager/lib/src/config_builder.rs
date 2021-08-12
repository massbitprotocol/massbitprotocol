/**
*** Objective of this file, is to build the IndexConfig from the user's Index Request
*** It will connect to IPFS to get the files and save them to storage
**/
// Generic dependencies
use std::path::PathBuf;
// Massbit dependencies
use crate::config::generate_random_hash;
use crate::ipfs::{get_ipfs_file_by_hash, read_config_file};
use crate::types::{IndexConfig, IndexIdentifier};
use adapter::setting::get_index_name;

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
    schema: PathBuf,
    config: PathBuf,
    mapping: PathBuf,
    hash: String,
}

impl Default for IndexConfigIpfsBuilder {
    fn default() -> IndexConfigIpfsBuilder {
        IndexConfigIpfsBuilder {
            schema: "".to_string().parse().unwrap(),
            config: "".to_string().parse().unwrap(),
            mapping: "".to_string().parse().unwrap(),
            hash: generate_random_hash(),
        }
    }
}

impl IndexConfigIpfsBuilder {
    pub async fn mapping(mut self, mapping: &String) -> IndexConfigIpfsBuilder {
        let file = &format!("{}{}", self.hash, ".so");
        let mut mapping = get_ipfs_file_by_hash(file, mapping).await;
        let mapping = ["./", &mapping].join("");
        self.mapping = PathBuf::from(mapping.to_string());
        self
    }

    pub async fn config(mut self, config: &String) -> IndexConfigIpfsBuilder {
        let file = &format!("{}{}", self.hash, ".yaml");
        let config = get_ipfs_file_by_hash(file, config).await;
        self.config = PathBuf::from(config);
        self
    }

    pub async fn schema(mut self, schema: &String) -> IndexConfigIpfsBuilder {
        let file = &format!("{}{}", self.hash, ".graphql");
        let schema = get_ipfs_file_by_hash(file, schema).await;
        self.schema = PathBuf::from(schema);
        self
    }

    pub fn build(self) -> IndexConfig {
        let config = read_config_file(&self.config);
        let name = get_index_name(&config);

        IndexConfig {
            schema: self.schema,
            config: self.config,
            mapping: self.mapping,
            identifier: IndexIdentifier {
                name: name.clone(),
                hash: self.hash.clone(),
                name_with_hash: format!("{}-{}", name, self.hash),
            },
        }
    }
}
