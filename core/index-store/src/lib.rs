extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
pub use graph::components::store::{
    EntityCollection, EntityFilter, EntityKey, EntityModification, EntityOrder, EntityRange,
    EntityType, StoredDynamicDataSource, StoreError, StoreEvent, WritableStore,
};
pub use graph::data::graphql::ext::ValueExt;
pub use graph::data::store::{Entity, Value};
use graph::data::subgraph::DeploymentHash;
pub use graph::prelude::q;
use lazy_static::lazy_static;

pub use struct_entity::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};

pub use crate::core::Store;
pub use crate::mapping::IndexerState;

pub mod core;
pub mod mapping;
pub mod postgres;
pub mod store;
pub mod struct_entity;
pub mod util;
lazy_static! {
    pub static ref COMPONENT_NAME: String = String::from("[Index-Store]");
    pub static ref DEPLOYMENT_HASH: DeploymentHash = DeploymentHash::new("_indexer").unwrap();
}
