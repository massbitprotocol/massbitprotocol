use super::orm::schema::solana_blocks::dsl as bl;
use super::orm::schema::solana_daily_stat_blocks::dsl as bl_stat;
use crate::orm::models::{SolanaBlock, SolanaDailyStatBlock};
use core::ops::Deref;
use diesel::r2d2::PooledConnection;
use jsonrpc_core::{Error, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use massbit::prelude::serde_json::{json, Value};
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
    #[rpc(name = "getBlockStatistic")]
    fn get_block_statistic(&self, offset: i64, limit: i64) -> JsonRpcResult<Value>;
    #[rpc(name = "getBlockList")]
    fn get_block_list(&self, offset: i64, limit: i64) -> JsonRpcResult<Value>;
    //Get block list from analytic database
    #[rpc(name = "getBlockDetail")]
    fn get_block_detail_db(&self, block_slot: i64) -> JsonRpcResult<Value>;
    #[rpc(name = "getBlockDetailChain")]
    fn get_block_detail_chain(&self, block_slot: Slot) -> JsonRpcResult<Value>;
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
    fn get_block_statistic(&self, offset: i64, limit: i64) -> JsonRpcResult<Value> {
        self.get_connection()
            .map_err(|_err| jsonrpc_core::Error::internal_error())
            .and_then(|conn| {
                bl_stat::solana_daily_stat_blocks
                    .order(bl_stat::date.desc())
                    .offset(offset)
                    .limit(limit)
                    .load::<SolanaDailyStatBlock>(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    })
                    .and_then(|vals| Ok(json!(vals)))
            })
    }

    fn get_block_list(&self, offset: i64, limit: i64) -> jsonrpc_core::Result<Value> {
        let block_res = self
            .get_connection()
            .map_err(|_err| jsonrpc_core::Error::internal_error())
            .and_then(|conn| {
                bl::solana_blocks
                    .order(bl::timestamp.desc())
                    .offset(offset)
                    .limit(limit)
                    .load::<SolanaBlock>(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    })
            });
        block_res.and_then(|blocks| Ok(json!(&blocks)))
    }

    fn get_block_detail_db(&self, block_slot: i64) -> jsonrpc_core::Result<Value> {
        log::info!("Get detail of block {}", block_slot);
        let start = Instant::now();

        let block_res = self
            .get_connection()
            .map_err(|_err| jsonrpc_core::Error::internal_error())
            .and_then(|conn| {
                bl::solana_blocks
                    .filter(bl::block_slot.eq(block_slot))
                    .first::<SolanaBlock>(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    })
            });
        log::info!("Get block from database in {:?}", start.elapsed());
        block_res.and_then(|block| Ok(json!(block)))
    }
    fn get_block_detail_chain(&self, block_slot: Slot) -> jsonrpc_core::Result<Value> {
        log::info!("Get detail of block {}", block_slot);
        let start = Instant::now();
        //Get block from net work
        match self.rpc_client.get_block(block_slot) {
            Ok(block) => {
                log::info!("Get block from network in {:?}", start.elapsed());
                Ok(json!(block))
            }
            Err(_e) => Err(Error::invalid_params("Block not found")),
        }
    }
}
