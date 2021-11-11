#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
use graph::data::subgraph::DeploymentHash;
use lazy_static::lazy_static;
use std::env;
pub mod core;
pub mod indexer;
pub mod mapping;
pub mod models;
pub mod postgres;
pub mod schema;
pub mod struct_entity;

lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[Index-Store]");
    pub static ref DATABASE_CONNECTION_STRING: String = env::var("DATABASE_CONNECTION_STRING")
        .unwrap_or(String::from("postgres://graph-node:let-me-in@localhost"));
    pub static ref DEPLOYMENT_HASH: DeploymentHash = DeploymentHash::new("_indexer").unwrap();
}
pub use crate::core::Store;
pub use crate::mapping::IndexerState;
pub use graph::components::store::{
    EntityCollection, EntityFilter, EntityKey, EntityModification, EntityOrder, EntityRange,
    EntityType, StoreError, StoreEvent, StoredDynamicDataSource, WritableStore,
};
pub use graph::data::graphql::ext::ValueExt;
pub use graph::data::store::{Attribute, Entity, Value};
pub use graph::prelude::q;
use massbit_common::prelude::diesel::{Connection, PgConnection};
pub use struct_entity::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};

pub fn establish_connection() -> PgConnection {
    PgConnection::establish(DATABASE_CONNECTION_STRING.as_str()).expect(&format!(
        "Error connecting to {}",
        DATABASE_CONNECTION_STRING.as_str()
    ))
}
