use crate::storage_adapter::StorageAdapter;
use massbit_common::prelude::diesel::{Connection, r2d2, insert_into, RunQueryDsl};
use massbit_common::prelude::diesel::r2d2::{ConnectionManager, Pool};
use std::cmp;
use diesel::PgConnection;
use diesel_dynamic_schema::table;
use massbit_common::prelude::diesel::result::Error;
use core::ops::Deref;

const MAX_POOL_SIZE : u32 = 10;

pub struct PostgresAdapter {
    pool: Pool<ConnectionManager<PgConnection>>
}

impl StorageAdapter for PostgresAdapter {
    // fn upsert(&self, table_name: String) -> Result<usize, anyhow::Error>{
    //     let table = table(table_name);
    //     let mut statement = insert_into(table)
    //         .values(values);
    //     if conflict_target.is_some() {
    //         if changes.is_some() {
    //             statement = statement.on_conflict(conflict_target.unwrap())
    //                 .do_update().set(changes.unwrap());
    //         } else {
    //             statement = statement.on_conflict(conflict_target.unwrap()).do_nothing();
    //         }
    //     }
    //     match self.pool.get() {
    //         Ok(conn) => {
    //             match statement.execute(conn.deref()) {
    //                 Ok(val) => Ok(val),
    //                 Err(err) => {
    //                     log::error!("{:?}", &err);
    //                     Err(err.into())
    //                 }
    //             }
    //         }
    //         Err(err) => {
    //             log::error!("{:?}", &err);
    //             Err(err.into())
    //         }
    //     }
    // }
}

#[derive(Default)]
pub struct PostgresAdapterBuilder {
    url: Option<String>,
    pool_size: u32,
}

impl PostgresAdapterBuilder {
    pub fn new() -> PostgresAdapterBuilder {
        // Set the minimally required fields of Foo.
        PostgresAdapterBuilder::default()
    }

    pub fn url(mut self, url: &String) -> PostgresAdapterBuilder {
        self.url = Some(url.clone());
        self
    }

    pub fn pool_size(mut self, pool_size: u32) -> PostgresAdapterBuilder {
        self.pool_size = pool_size;
        self
    }

    pub fn build(self) -> PostgresAdapter {

        let pool_size = cmp::max(self.pool_size, MAX_POOL_SIZE);
        let conn_pool = create_r2d2_connection_pool::<PgConnection>(self.url.unwrap().as_str(), pool_size);
        PostgresAdapter {
            pool: conn_pool
        }
    }
}

pub fn create_r2d2_connection_pool<T:'static + Connection>(db_url: &str, pool_size: u32) -> r2d2::Pool<ConnectionManager<T>> {
    let manager = ConnectionManager::<T>::new(db_url);
    r2d2::Pool::builder().max_size(pool_size).build(manager).expect("Can not create connection pool")
}