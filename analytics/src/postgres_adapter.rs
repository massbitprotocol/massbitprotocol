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
use graph::prelude::{Value, StoreError};
use graph::components::subgraph::Entity;
use massbit_common::prelude::diesel::query_builder::{QueryId, QueryFragment, AstPass};
use massbit_common::prelude::diesel::pg::Pg;

const MAX_POOL_SIZE : u32 = 10;

pub struct PostgresAdapter {
    pool: Pool<ConnectionManager<PgConnection>>
}

impl StorageAdapter for PostgresAdapter {
    // fn get_connection(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, r2d2::Error> {
    //     self.pool.get()
    // }
    fn upsert(&self, table_name: &str, mut entities: Vec<Entity>) -> Result<(), anyhow::Error> {
        println!("Upsert value {:?} into {:?}", &entities, &table_name);


        match self.pool.get() {
            Ok(conn) => {
                let upsert_query = UpsertQuery::new(table_name, &entities)?;
                match upsert_query.execute(conn.deref()) {
                    Ok(val) => Ok(()),
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
        // let query = format!(
        //     "insert into {}(hash, number, parent_hash, data) \
        //              values ($1, $2, $3, $4) \
        //                  on conflict(hash) \
        //                  do update set number = $2, parent_hash = $3, data = $4",
        //     blocks.qname,
        // );
        // let parent_hash = block.block.parent_hash;
        // let hash = block.block.hash.unwrap();
        // sql_query(query)
        //     .bind::<Bytea, _>(hash.as_bytes())
        //     .bind::<BigInt, _>(number)
        //     .bind::<Bytea, _>(parent_hash.as_bytes())
        //     .bind::<Jsonb, _>(data)
        //     .execute(conn)?;
        // let table = table(table_name);
        // let mut statement = insert_into(table)
        //     .values(values);
        // if conflict_target.is_some() {
        //     if changes.is_some() {
        //         statement = statement.on_conflict(conflict_target.unwrap())
        //             .do_update().set(changes.unwrap());
        //     } else {
        //         statement = statement.on_conflict(conflict_target.unwrap()).do_nothing();
        //     }
        // }
    }
}
#[derive(Debug)]
pub struct UpsertQuery<'a>{
    table: &'a str,
    entities: &'a Vec<Entity>,
    columns: Vec<String>
}
impl<'a> UpsertQuery<'a> {
    pub fn new(
        table: &'a str,
        entities: &'a Vec<Entity>
    ) -> Result<UpsertQuery<'a>, StoreError> {
        let columns = Vec::default();
        Ok(UpsertQuery {
            table,
            entities,
            columns,
        })
    }
}
impl<'a> QueryFragment<Pg> for UpsertQuery<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        // Construct a query
        //   insert into schema.table(column, ...)
        //   values
        //   (a, b, c),
        //   (d, e, f)
        //   [...]
        //   (x, y, z)
        //   on conflict (name)
        //   do
        //   update set email = EXCLUDED.email || ';' || customers.email;
        //
        // and convert and bind the entity's values into it
        out.push_sql("insert into ");
        out.push_sql(self.table);

        out.push_sql("(");

        for &column in &self.unique_columns {
            out.push_identifier(column.name.as_str())?;
            out.push_sql(", ");
        }
        //out.push_identifier(BLOCK_RANGE_COLUMN)?;

        out.push_sql(") values\n");

        // Use a `Peekable` iterator to help us decide how to finalize each line.
        let mut iter = self.entities.iter().map(|entity| entity).peekable();
        while let Some(entity) = iter.next() {
            out.push_sql("(");
            for column in &self.columns {
                // If the column name is not within this entity's fields, we will issue the
                // null value in its place
                if let Some(value) = entity.get(&column) {
                    QueryValue(value, &column.column_type).walk_ast(out.reborrow())?;
                } else {
                    out.push_sql("null");
                }
                out.push_sql(", ");
            }
            let block_range: BlockRange = (self.block..).into();
            //out.push_bind_param::<Range<Integer>, _>(&block_range)?;
            out.push_sql(")");

            // finalize line according to remaining entities to insert
            if iter.peek().is_some() {
                out.push_sql(",\n");
            }
        }
        out.push_sql("\nreturning ");
        out.push_sql(PRIMARY_KEY_COLUMN);
        out.push_sql("::text");

        Ok(())
    }
}

impl<'a> QueryId for UpsertQuery<'a> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}
impl<'a, Conn> RunQueryDsl<Conn> for UpsertQuery<'a> {}

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