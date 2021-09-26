use crate::schema::indexers::columns::network;
use crate::store::BlockStore as BlockStoreTrait;
use crate::util::create_r2d2_connection_pool;
use core::ops::Deref;
use diesel::pg::types::sql_types::Jsonb;
use diesel::table;
use diesel_migrations::embed_migrations;
use graph::components::ethereum::EthereumBlock as FullEthereumBlock;
use massbit_common::prelude::anyhow::Error;
use massbit_common::prelude::diesel::{
    r2d2::{Pool, PooledConnection},
    ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use massbit_common::prelude::r2d2_diesel::ConnectionManager;
use massbit_common::prelude::{r2d2, serde_json};
use schema::ethereum_block;
use serde::{Deserialize, Serialize};

pub type PooledPgConnection = PooledConnection<ConnectionManager<PgConnection>>;
pub type PoolPgConnection = Pool<ConnectionManager<PgConnection>>;
#[derive(Clone)]
pub struct EthereumBlockStore {
    pub pool: PoolPgConnection,
}
embed_migrations!("./migrations/ethereum");

impl EthereumBlockStore {
    pub fn new(db_url: &str) -> EthereumBlockStore {
        log::info!("Create EthereumBlockStore with url {}", db_url);
        let pool = create_r2d2_connection_pool::<PgConnection>(db_url);
        match pool.get() {
            Ok(conn) => {
                let result =
                    embedded_migrations::run_with_output(conn.deref(), &mut std::io::stdout());
                log::info!("{:?}", &result);
            }
            Err(_) => {}
        };
        EthereumBlockStore { pool }
    }
    pub fn get_connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, r2d2::Error> {
        self.pool.get()
    }
}

impl BlockStoreTrait for EthereumBlockStore {
    fn get_latest_block_number(&self) -> Option<u64> {
        match self.get_connection() {
            Ok(conn) => {
                if let Ok(block_numbers) = ethereum_block::table
                    .select(ethereum_block::number)
                    .order(ethereum_block::number.desc())
                    .limit(1)
                    .load::<i64>(conn.deref())
                {
                    match block_numbers.get(0) {
                        Some(val) => Some(*val as u64),
                        None => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
    fn get_earliest_block_number(&self) -> Option<u64> {
        match self.get_connection() {
            Ok(conn) => {
                if let Ok(block_numbers) = ethereum_block::table
                    .select(ethereum_block::number)
                    .order(ethereum_block::number.asc())
                    .limit(1)
                    .load::<i64>(conn.deref())
                {
                    match block_numbers.get(0) {
                        Some(val) => Some(*val as u64),
                        None => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn store_full_ethereum_blocks(
        &self,
        full_blocks: &Vec<FullEthereumBlock>,
        network_name: String,
    ) -> Result<(), Error> {
        if let Ok(conn) = self.get_connection() {
            let values = full_blocks
                .iter()
                .map(|block| EthereumBlock::from_full_ethereum_block(block, network_name.clone()))
                .collect::<Vec<EthereumBlock>>();
            let result = diesel::insert_into(schema::ethereum_block::table)
                .values(&values)
                .execute(conn.deref());
            match result {
                Ok(_) => {}
                Err(err) => {
                    log::error!("{:?}", &err)
                }
            }
        }
        Ok(())
    }
}

#[derive(FromSqlRow, AsExpression, Debug, Insertable, Serialize, Deserialize)]
#[sql_type = "Jsonb"]
#[table_name = "ethereum_block"]
pub struct EthereumBlock {
    pub hash: String,
    pub number: i64,
    pub parent_hash: String,
    network_name: String,
    data: serde_json::Value,
}

impl EthereumBlock {
    fn from_full_ethereum_block(block: &FullEthereumBlock, network_name: String) -> Self {
        let parent_hash = format!("{:x}", block.block.parent_hash);
        let hash = format!("{:x}", block.block.hash.unwrap());
        let number = block.block.number.unwrap().as_u64() as i64;
        let data = serde_json::to_value(&block).expect("Failed to serialize block");
        EthereumBlock {
            hash,
            number,
            parent_hash,
            network_name,
            data,
        }
    }
}
// impl FromSql<Jsonb, Pg> for EthereumBlock {
//     fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
//         let value = <serde_json::Value as FromSql<Jsonb, Pg>>::from_sql(bytes)?;
//         Ok(serde_json::from_value(value)?)
//     }
// }
//
// impl ToSql<Jsonb, Pg> for EthereumBlock {
//     fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
//         let value = serde_json::to_value(self)?;
//         <serde_json::Value as ToSql<Jsonb, Pg>>::to_sql(out)
//     }
// }
pub mod schema {
    table! {
        ethereum_block (hash) {
            hash -> Varchar,
            number -> BigInt,
            parent_hash -> Nullable<Varchar>,
            network_name -> Varchar,
            data -> Jsonb,
        }
    }
}
