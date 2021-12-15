#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate diesel;
extern crate diesel_dynamic_schema;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel_derive_enum;
extern crate postgres;

pub mod advisory_lock;
pub mod block_range;
pub mod catalog;
pub mod connection_pool;
pub mod deployment;
pub mod deployment_store;
pub mod indexer_store;
pub mod primary;
pub mod query_store;
pub mod relational;
pub mod relational_queries;
pub mod sql_value;
pub mod store;

pub use self::indexer_store::{unused, DeploymentPlacer, IndexerStore, Shard, PRIMARY_SHARD};

pub mod command_support {
    pub mod catalog {
        pub use crate::catalog::{account_like, set_account_like};
        pub use crate::primary::Site;
    }
    pub use crate::relational::{Catalog, Column, ColumnType, Layout, SqlName};
}
use indexer_orm::models::Namespace;
use massbit_common::prelude::diesel::{
    r2d2::{self, ConnectionManager},
    Connection,
};

pub fn create_r2d2_connection_pool<T: 'static + Connection>(
    db_url: &str,
    pool_size: u32,
) -> r2d2::Pool<ConnectionManager<T>> {
    let manager = ConnectionManager::<T>::new(db_url);
    r2d2::Pool::builder()
        .max_size(pool_size)
        .build(manager)
        .expect("Can not create connection pool")
}
