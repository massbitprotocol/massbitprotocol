use crate::block_api::{RpcBlocks, RpcBlocksImpl};
use crate::transaction_api::{RpcTransactions, RpcTransactionsImpl};
use crate::{CONNECTION_POOL_SIZE, DATABASE_URL};
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use massbit_common::prelude::diesel::PgConnection;
use massbit_store_postgres::helper::create_r2d2_connection_pool;
use solana_client::rpc_client::RpcClient;
use std::sync::Arc;

pub fn create_solana_api_io(solana_client: Arc<RpcClient>) -> IoHandler {
    let mut io = IoHandler::default();
    let connection_pool =
        create_r2d2_connection_pool::<PgConnection>(DATABASE_URL.as_str(), *CONNECTION_POOL_SIZE);

    let rpc_block = RpcBlocksImpl::new(solana_client.clone(), connection_pool.clone());
    io.extend_with(rpc_block.to_delegate());
    let rpc_transaction = RpcTransactionsImpl::new(solana_client.clone(), connection_pool.clone());
    io.extend_with(rpc_transaction.to_delegate());
    io
}
