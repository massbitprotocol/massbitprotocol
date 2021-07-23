use std::path::PathBuf;

pub struct IndexConfig {
    model: String,
    table: String,
    config: PathBuf,
    mapping: String,
    query: String,
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

#[derive(Default)]
pub struct IndexConfigBuilder {
    model: String,
    table: String,
    config: PathBuf,
    mapping: String,
    query: String,
}

impl IndexConfigBuilder {
    pub fn model(mut self, model: String) -> IndexConfigBuilder {
        self.model = model;
        self
    }

    pub fn query(mut self, query: String) -> IndexConfigBuilder {
        self.model = query;
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

