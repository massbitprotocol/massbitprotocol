use crate::store::BlockStore as BlockStoreTrait;
use diesel_migrations::embed_migrations;
use diesel::table;
use graph::components::ethereum::EthereumBlock as FullEthereumBlock;
use crate::util::create_r2d2_connection_pool;
use massbit_common::prelude::anyhow::Error;
use massbit_common::prelude::r2d2_diesel::ConnectionManager;
use massbit_common::prelude::r2d2;
use massbit_common::prelude::diesel::{
    PgConnection,
    r2d2::{Pool, PooledConnection}
};

pub type PooledPgConnection = PooledConnection<ConnectionManager<PgConnection>>;
pub type PoolPgConnection = Pool<ConnectionManager<PgConnection>>;
#[derive(Clone)]
pub struct EthereumBlockStore {
    pub pool : PoolPgConnection
}
embed_migrations!("./migrations/ethereum");

impl EthereumBlockStore {
    pub fn new(db_url : &str) -> EthereumBlockStore {
        let pool = create_r2d2_connection_pool::<PgConnection>(db_url);
        match pool.get() {
            Ok(conn) => {
                embedded_migrations::run(&conn);
            }
            Err(_) => {}
        };
        EthereumBlockStore {
            pool
        }
    }
    pub fn get_connection(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, r2d2::Error> {
        self.pool.get()
    }
}

impl BlockStoreTrait for EthereumBlockStore {
    fn store_full_ethereum_blocks(&self, full_block: &Vec<FullEthereumBlock>) -> Result<usize, Error> {
        if let Ok(conn) = self.get_connection() {
            // let values =
            // diesel::insert_into(ethereum_block::table)
            //     .values()
            //     .execute(&conn);
        }
        todo!()
    }
}

#[derive(FromSqlRow, AsExpression, Debug, Serialize, Deserialize)]
#[sql_type = "Jsonb"]
pub struct EthereumBlock {
    pub hash: String,
    pub number: i64,
    pub parent_hash: String,
    network_name: String,
    data: String
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

table! {
    ethereum_block (hash) {
        hash -> Text,
        number -> Int8,
        parent_hash -> Text,
        network_name -> Text,
        data -> Text
    }
}