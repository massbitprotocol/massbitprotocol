use crate::storage_adapter::StorageAdapter;
use massbit_common::prelude::diesel::{Connection, r2d2, insert_into, RunQueryDsl, sql_query, QueryResult};
use massbit_common::prelude::diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use std::cmp;
use diesel::PgConnection;
use diesel_dynamic_schema::table;
use massbit_common::prelude::diesel::result::{Error as DieselError, Error};


use core::ops::Deref;
use massbit_common::prelude::diesel::pg::types::sql_types::Jsonb;
use std::collections::HashMap;
use graph::prelude::{Value, StoreError, Entity};
use massbit_common::prelude::diesel::query_builder::{QueryId, QueryFragment, AstPass};
use massbit_common::prelude::diesel::pg::Pg;
use crate::postgres_queries::{UpsertQuery, UpsertConflictFragment};
use crate::relational::{Column, Table};
use std::time::Instant;
use crate::models::CommandData;

const MAX_POOL_SIZE : u32 = 10;

pub struct PostgresAdapter {
    pool: Pool<ConnectionManager<PgConnection>>
}

impl StorageAdapter for PostgresAdapter {
    fn upsert(&self, table: &Table, columns: &Vec<Column>, entities: &Vec<Entity>, conflict_fragment: &Option<UpsertConflictFragment>) -> Result<(), anyhow::Error> {
        let start = Instant::now();
        match self.pool.get() {
            Ok(conn) => {
                let upsert_query = UpsertQuery::new(table, columns, entities, conflict_fragment)?;
                match upsert_query.execute(conn.deref()) {
                    Ok(val) => {
                        log::debug!("Upsert {} entities into table {} in {:?}", entities.len(), table.name, start.elapsed());
                        Ok(())
                    },
                    Err(err) => {
                        log::error!("{:?}", &err);
                        Err(err.into())
                    }
                }
            }
            Err(err) => {
                log::error!("{:?}", &err);
                Err(err.into())
            }
        }
    }
    fn transact_upserts(&self, commands: Vec<CommandData>) -> Result<(), anyhow::Error> {
        let start = Instant::now();
        match self.pool.get() {
            Ok(conn) => {
                conn.transaction::<(), anyhow::Error,_>(||{
                    commands.iter().for_each(|cmd| {
                        let upsert_query = UpsertQuery::from(cmd);
                        match upsert_query.execute(conn.deref()) {
                            Ok(val) => {
                                log::debug!("Upsert {} entities into table {} in {:?}", cmd.values.len(), cmd.table.name, start.elapsed());
                            },
                            Err(err) => {
                                log::error!("{:?}", &err);
                            }
                        }
                    });
                    Ok(())
                })
            }
            Err(err) => {
                log::error!("{:?}", &err);
                Err(err.into())
            }
        }
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