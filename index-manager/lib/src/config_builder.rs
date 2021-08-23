/**
 *** Objective of this file, is to build the IndexConfig from the user's Index Request
 *** It will connect to IPFS to get the files and save them to storage
 **/
// Generic dependencies
use std::path::PathBuf;
use lazy_static::lazy_static;

// Massbit dependencies
use crate::config::{
    generate_mapping_name_and_type, generate_random_hash, get_index_name, get_mapping_language,
};
use crate::ipfs::{get_ipfs_file_by_hash, read_config_file};
use crate::types::{Abi, IndexConfig, IndexIdentifier};

lazy_static! {
    static ref GENERATED_FOLDER: String = String::from("index-manager/generated");
}

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
/*******************************************************************************
  IndexConfigIpfsBuilder

  Description:
  To build the index config based on the config from IPFS
*******************************************************************************/
pub struct IndexConfigIpfsBuilder {
    schema: PathBuf,
    config: PathBuf,
    mapping: PathBuf,
    abi: PathBuf,
    hash: String,
}

impl Default for IndexConfigIpfsBuilder {
    fn default() -> IndexConfigIpfsBuilder {
        IndexConfigIpfsBuilder {
            schema: Default::default(),
            config: Default::default(),
            mapping: Default::default(),
            abi: Default::default(),
            hash: generate_random_hash(),
        }
    }
}

impl IndexConfigIpfsBuilder {
    // Call to IPFS and download mapping to local storage
    // Mapping file type is decided by the self.config value
    pub async fn mapping(mut self, mapping: &String) -> IndexConfigIpfsBuilder {
        assert_eq!(
            self.config.as_os_str().is_empty(),
            false,
            "Config should be provided before mapping and schema"
        );
        let config_value = read_config_file(&self.config);
        let file_name = generate_mapping_name_and_type(&config_value);
        let mapping = get_ipfs_file_by_hash(&file_name, &self.hash, mapping).await;
        self.mapping = PathBuf::from(&mapping);
        self
    }

    // Call to IPFS and download config to local storage
    pub async fn config(mut self, config: &String) -> IndexConfigIpfsBuilder {
        let config = get_ipfs_file_by_hash(
            &String::from("project.yaml"),
            &self.hash,
            config,
        ).await;
        self.config = PathBuf::from(&config);
        self
    }

    // Call to IPFS and download schema to local storage
    pub async fn schema(mut self, schema: &String) -> IndexConfigIpfsBuilder {
        let schema = get_ipfs_file_by_hash(
            &String::from("schema.graphql"),
            &self.hash,
            schema,
        ).await;
        self.schema = PathBuf::from(&schema);
        self
    }

    // Call to IPFS and download ABIs to local storage
    pub async fn abi(mut self, abi: &Option<Vec<Abi>>) -> IndexConfigIpfsBuilder {
        match abi {
            Some(v) => {
                for object in v {
                    let abi = get_ipfs_file_by_hash(&object.name, &self.hash, &object.hash).await;
                    self.abi = PathBuf::from(&abi)
                }
            }
            None => println!(".SO mapping or this index type doesn't support ABIs"),
        }
        self
    }

    pub fn build(self) -> IndexConfig {
        let config = read_config_file(&self.config);
        let name = get_index_name(&config);

        IndexConfig {
            schema: self.schema,
            config: self.config,
            mapping: self.mapping,
            abi: Default::default(),
            identifier: IndexIdentifier {
                name: name.clone(),
                hash: self.hash.clone(),
                name_with_hash: format!("{}-{}", name, self.hash),
            },
        }
    }
}

/*******************************************************************************
  IndexConfigLocalBuilder

  Description:
  To build the index config based on the name and hash from the indexers table
*******************************************************************************/
impl Default for IndexConfigLocalBuilder {
    fn default() -> IndexConfigLocalBuilder {
        IndexConfigLocalBuilder {
            schema: Default::default(),
            config: Default::default(),
            mapping: Default::default(),
            hash: generate_random_hash(),
        }
    }
}

pub struct IndexConfigLocalBuilder {
    schema: PathBuf,
    config: PathBuf,
    mapping: PathBuf,
    hash: String,
}

impl IndexConfigLocalBuilder {
    pub async fn mapping(mut self, name: &String) -> IndexConfigLocalBuilder {
        let mapping = [GENERATED_FOLDER.as_str(), name, ".so"].join("");
        self.mapping = PathBuf::from(mapping.to_string());
        self
    }

    pub async fn config(mut self, name: &String) -> IndexConfigLocalBuilder {
        let config = [GENERATED_FOLDER.as_str(), name, ".yaml"].join("");
        self.config = PathBuf::from(config);
        self
    }

    pub async fn schema(mut self, name: &String) -> IndexConfigLocalBuilder {
        let schema = [GENERATED_FOLDER.as_str(), name, ".graphql"].join("");
        self.schema = PathBuf::from(schema);
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            schema: self.schema,
            config: self.config,
            mapping: self.mapping,
            abi: Default::default(),
            identifier: IndexIdentifier {
                // TODO: populate with the value from the indexer query result
                name: Default::default(),
                hash: Default::default(),
                name_with_hash: Default::default(),
            },
        }
    }
}
