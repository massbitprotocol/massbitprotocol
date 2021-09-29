#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate diesel;
extern crate diesel_dynamic_schema;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel_derive_enum;
extern crate fallible_iterator;
extern crate futures;
extern crate graphql_parser;
extern crate inflector;
extern crate lazy_static;
extern crate lru_time_cache;
extern crate massbit;
extern crate postgres;
extern crate serde;
extern crate uuid;

mod advisory_lock;
mod block_range;
mod catalog;
pub mod connection_pool;
mod deployment;
mod deployment_store;
mod detail;
mod dynds;
mod indexer_store;
mod primary;
mod relational;
mod relational_queries;
mod sql_value;

pub use self::indexer_store::{IndexerStore, Shard, PRIMARY_SHARD};

/// This module is only meant to support command line tooling. It must not
/// be used in 'normal' code
pub mod command_support {
    pub mod catalog {
        pub use crate::primary::Site;
    }
}
