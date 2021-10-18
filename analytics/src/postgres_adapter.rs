use crate::storage_adapter::StorageAdapter;
use diesel::PgConnection;
use massbit_common::prelude::diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use massbit_common::prelude::diesel::{r2d2, Connection, RunQueryDsl};
use std::cmp;

use crate::models::CommandData;
use crate::postgres_queries::{UpsertConflictFragment, UpsertQuery};
use crate::relational::Table;
use crate::MAX_POOL_SIZE;
use core::ops::Deref;
use massbit::data::store::Value;
use massbit::prelude::Entity;
use massbit_common::prelude::r2d2::Error;
use std::collections::HashMap;
use std::time::Instant;

pub struct PostgresAdapter {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl StorageAdapter for PostgresAdapter {
    fn insert(
        &self,
        _table_name: &str,
        _value: HashMap<&str, Value>,
    ) -> Result<(), massbit::prelude::Error> {
        todo!()
    }

    fn upsert(
        &self,
        table: &Table,
        entities: &Vec<Entity>,
        conflict_fragment: &Option<UpsertConflictFragment>,
    ) -> Result<(), anyhow::Error> {
        let start = Instant::now();
        if entities.len() > 0 {
            match self.pool.get() {
                Ok(conn) => {
                    let upsert_query = UpsertQuery::new(table, entities, conflict_fragment)?;
                    match upsert_query.execute(conn.deref()) {
                        Ok(_val) => {
                            log::info!(
                                "Upsert {} entities into table {} in {:?}",
                                entities.len(),
                                table.name,
                                start.elapsed()
                            );
                            Ok(())
                        }
                        Err(err) => {
                            log::error!(
                                "Error while insert into table {:?} {:?}",
                                &table.name,
                                &err
                            );
                            log::error!("{:?}", entities);
                            Err(err.into())
                        }
                    }
                }
                Err(err) => {
                    log::error!("{:?}", &err);
                    Err(err.into())
                }
            }
        } else {
            Ok(())
        }
    }
    fn transact_upserts(&self, commands: Vec<CommandData>) -> Result<(), anyhow::Error> {
        let start = Instant::now();
        match self.pool.get() {
            Ok(conn) => conn.transaction::<(), anyhow::Error, _>(|| {
                commands.iter().for_each(|cmd| {
                    let upsert_query = UpsertQuery::from(cmd);
                    match upsert_query.execute(conn.deref()) {
                        Ok(_val) => {
                            log::info!(
                                "Upsert {} entities into table {} in {:?}",
                                cmd.values.len(),
                                cmd.table.name,
                                start.elapsed()
                            );
                        }
                        Err(err) => {
                            log::error!(
                                "Error while insert into table {:?} {:?}",
                                &cmd.table.name,
                                &err
                            );
                        }
                    }
                });
                Ok(())
            }),
            Err(err) => {
                log::error!("{:?}", &err);
                Err(err.into())
            }
        }
    }
}

impl PostgresAdapter {
    pub fn get_connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, Error> {
        self.pool.get()
    }
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
        let conn_pool =
            create_r2d2_connection_pool::<PgConnection>(self.url.unwrap().as_str(), pool_size);
        PostgresAdapter { pool: conn_pool }
    }
}

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
