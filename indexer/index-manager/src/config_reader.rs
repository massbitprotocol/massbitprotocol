use std::path::PathBuf;
use crate::helper::get_raw_query_from_ipfs;
use crate::types::DeployType;

pub struct IndexConfig {
    pub model: String,
    pub table: String,
    pub config: PathBuf,
    pub mapping: String,
    pub query: String,
}

/**
*** Usage: let foo: Foo = FooBuilder::default().name(String::from("abc")).build();
*** Real example: https://github.com/graphprotocol/rust-web3/blob/3aac17f719b99494793111fd00a4505fe4670ca2/src/types/log.rs#L103
*** Advantages:
***  - Separates methods for building from other methods.
***  - Prevents proliferation of constructors
***  - Can be used for one-liner initialisation as well as more complex construction.
*** Note:
***  - I think this is useful when there's too many complex check that needs to be done and we want to hide it from the main logic
*** Reference: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
**/
impl IndexConfig {
    pub fn builder() -> IndexConfigBuilder {
        IndexConfigBuilder::default()
    }
}

pub struct IndexConfigBuilder {
    model: String,
    table: String,
    config: PathBuf,
    mapping: String,
    query: String,
    pub deploy_type: DeployType,
}

impl Default for IndexConfigBuilder {
    fn default() -> IndexConfigBuilder {
        IndexConfigBuilder {
            model: "".to_string(),
            table: "".to_string(),
            config: Default::default(),
            mapping: "".to_string(),
            query: "".to_string(),
            deploy_type: DeployType::Ipfs,
        }
    }
}

impl IndexConfigBuilder {
    pub fn model(mut self, model: String) -> IndexConfigBuilder {
        self.model = model;
        self
    }

    pub async fn query(mut self, query: String) -> IndexConfigBuilder {
        self.query = get_raw_query_from_ipfs(&query).await;
        self
    }

    pub fn deploy_type(mut self, deploy_type: DeployType) -> IndexConfigBuilder {
        self.deploy_type = deploy_type;
        self
    }

    pub fn build(self) -> IndexConfig {
        IndexConfig {
            model: self.model,
            table: self.table,
            config: self.config,
            mapping: self.mapping,
            query: self.query,
        }
    }
}
