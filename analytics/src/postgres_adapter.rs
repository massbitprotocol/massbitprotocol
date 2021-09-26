use crate::storage_adapter::StorageAdapter;
use massbit_common::prelude::r2d2;
use massbit_common::prelude::r2d2_diesel::ConnectionManager;
use massbit_common::prelude::diesel::{Connection, r2d2, insert_into, RunQueryDsl};
use massbit_common::prelude::diesel::r2d2::{ConnectionManager, Pool};
use std::cmp;
use diesel::PgConnection;

const MAX_POOL_SIZE : u32 = 10;
pub struct PostgresAdapter {
    pool: Pool<ConnectionManager<PgConnection>>
}

impl StorageAdapter for PostgresAdapter {
    fn upsert(values : &Vec<>) {
        insert_into(b::table)
            .values(values.clone())
            .on_conflict(b::hash)
            .do_nothing()
            .execute(conn);
        diesel::insert_into(users)
            .values(&vec![user2, user3])
            .on_conflict(id)
            .do_update()
            .set(name.eq(excluded(name)))
            .execute(&conn)
        match insert_into(network_state::table)
            .values((network_state::chain.eq(CHAIN.clone()),
                     network_state::network.eq(network.clone().unwrap_or(DEFAULT_NETWORK.to_string())),
                     network_state::got_block.eq(ethereum_block.block_number.unwrap()))
            )
            .on_conflict((network_state::chain, network_state::network))
            .do_update()
            .set(network_state::got_block.eq(excluded(network_state::got_block)))
            .execute(&conn) {
            Ok(_) => {}
            Err(err) => log::error!("{:?}",&err)
        };
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
        let conn_pool = create_r2d2_connection_pool::<PgConnection>();
        PostgresAdapter {
            pool: conn_pool
        }
    }
}

pub fn create_r2d2_connection_pool<T:'static + Connection>(db_url: &str) -> r2d2::Pool<ConnectionManager<T>> {
    let manager = ConnectionManager::<T>::new(db_url);
    r2d2::Pool::builder().max_size(pool_size).build(manager).expect("Can not create connection pool")
}