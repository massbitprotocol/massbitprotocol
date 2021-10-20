use super::orm::schema::solana_blocks::dsl;
use crate::orm::models::SolanaBlock;
use core::ops::Deref;
use diesel::r2d2::PooledConnection;
use jsonrpc_core::{Error, Params, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use massbit::prelude::serde_json;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{
    r2d2, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use solana_client::rpc_client::RpcClient;
use solana_program::clock::Slot;
use std::sync::Arc;
use tokio::time::Instant;

#[rpc]
pub trait RpcBlocks {
    #[rpc(name = "block/lasts")]
    fn get_last_blocks(&self, limit: i64) -> JsonRpcResult<String>;
    #[rpc(name = "block/detail_db")]
    fn get_dbblock_detail(&self, block_slot: i64) -> JsonRpcResult<String>;
    #[rpc(name = "block/detail_net")]
    fn get_netblock_detail(&self, block_slot: Slot) -> JsonRpcResult<String>;
}

pub struct RpcBlocksImpl {
    pub rpc_client: Arc<RpcClient>,
    pub connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}
impl RpcBlocksImpl {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        RpcBlocksImpl {
            rpc_client,
            connection_pool,
        }
    }
    pub fn get_connection(
        &self,
    ) -> Result<
        PooledConnection<ConnectionManager<PgConnection>>,
        massbit_common::prelude::r2d2::Error,
    > {
        self.connection_pool.get()
    }
}
impl RpcBlocks for RpcBlocksImpl {
    fn get_last_blocks(&self, limit: i64) -> jsonrpc_core::Result<String> {
        let block_res = self
            .get_connection()
            .map_err(|_err| jsonrpc_core::Error::internal_error())
            .and_then(|conn| {
                dsl::solana_blocks
                    .order(dsl::timestamp.desc())
                    .limit(limit)
                    .load::<SolanaBlock>(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    })
            });
        block_res.and_then(|blocks| {
            serde_json::to_string(&blocks).map_err(|_err| jsonrpc_core::Error::parse_error())
        })
    }

    fn get_dbblock_detail(&self, block_slot: i64) -> jsonrpc_core::Result<String> {
        log::info!("Get detail of block {}", block_slot);
        let start = Instant::now();

        let block_res = self
            .get_connection()
            .map_err(|_err| jsonrpc_core::Error::internal_error())
            .and_then(|conn| {
                dsl::solana_blocks
                    .filter(dsl::block_slot.eq(block_slot))
                    .first::<SolanaBlock>(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    })
            });
        log::info!("Get block from database in {:?}", start.elapsed());
        block_res.and_then(|block| {
            serde_json::to_string(&block).map_err(|_err| jsonrpc_core::Error::parse_error())
        })
    }
    fn get_netblock_detail(&self, block_slot: Slot) -> jsonrpc_core::Result<String> {
        log::info!("Get detail of block {}", block_slot);
        let start = Instant::now();
        //Get block from net work
        match self.rpc_client.get_block(block_slot) {
            Ok(block) => {
                log::info!("Get block from network in {:?}", start.elapsed());

                match serde_json::to_string(&block) {
                    Ok(value) => Ok(value),
                    Err(_) => Ok(String::from("")),
                }
            }
            Err(e) => Err(Error::invalid_params("Block not found")),
        }
    }
}
