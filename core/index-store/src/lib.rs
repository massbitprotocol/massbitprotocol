#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
use graph::data::subgraph::DeploymentHash;
use lazy_static::lazy_static;
pub mod core;
pub mod mapping;
pub mod models;
pub mod postgres;
pub mod schema;
pub mod struct_entity;
lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[Index-Store]");
    pub static ref DEPLOYMENT_HASH: DeploymentHash = DeploymentHash::new("_indexer").unwrap();
}
pub use crate::core::Store;
pub use crate::mapping::IndexerState;
pub use graph::components::store::{
    EntityCollection, EntityFilter, EntityKey, EntityModification, EntityOrder, EntityRange,
    EntityType, StoreError, StoreEvent, StoredDynamicDataSource, WritableStore,
};
pub use graph::data::graphql::ext::ValueExt;
pub use graph::data::store::{Entity, Value};
pub use graph::prelude::q;
pub use struct_entity::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
