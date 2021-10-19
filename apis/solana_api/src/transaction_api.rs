use jsonrpc_core::{Params, Result};
use jsonrpc_derive::rpc;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{r2d2, PgConnection};
use solana_client::rpc_client::RpcClient;
use std::sync::Arc;

#[rpc]
pub trait RpcTransactions {}

pub struct RpcTransactionsImpl {
    pub rpc_client: Arc<RpcClient>,
    pub connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}
impl RpcTransactionsImpl {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        RpcTransactionsImpl {
            rpc_client,
            connection_pool,
        }
    }
}

impl RpcTransactions for RpcTransactionsImpl {}
