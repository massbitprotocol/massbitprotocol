use crate::account_api::{RpcAccounts, RpcAccountsImpl};
use crate::block_api::{RpcBlocks, RpcBlocksImpl};
use crate::transaction_api::{RpcTransactions, RpcTransactionsImpl};
use crate::{CONNECTION_POOL_SIZE, DATABASE_URL};
use core::sync::atomic;
use jsonrpc_core::futures_util::{future::Either, FutureExt};
use jsonrpc_core::{
    middleware, FutureResponse, MetaIoHandler, Metadata, Middleware, Request, Response,
};
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use massbit::prelude::tokio::time::Instant;
use massbit_common::prelude::diesel::PgConnection;
use massbit_store_postgres::helper::create_r2d2_connection_pool;
use solana_client::rpc_client::RpcClient;
use std::future::Future;
use std::sync::Arc;

// Todo: try to add response header
// #[derive(Clone, Debug)]
// struct ApiMeta {}
// impl Metadata for ApiMeta {}
//
// #[derive(Default)]
// struct SolanaApiMiddleware {}
// impl Middleware<ApiMeta> for SolanaApiMiddleware {
//     type Future = FutureResponse;
//     type CallFuture = middleware::NoopCallFuture;
//
//     fn on_request<F, X>(&self, request: Request, meta: ApiMeta, next: F) -> Either<Self::Future, X>
//     where
//         F: FnOnce(Request, ApiMeta) -> X + Send,
//         X: Future<Output = Option<Response>> + Send + 'static,
//     {
//         let start = Instant::now();
//         //let request_number = self.0.fetch_add(1, atomic::Ordering::SeqCst);
//         //println!("Processing request {}: {:?}, {:?}", request_number, request, meta);
//         Either::Left(Box::pin(next(request, meta).map(move |mut res| {
//             println!("Processing took: {:?}", start.elapsed());
//             res
//         })))
//     }
// }

pub fn create_solana_api_io(solana_client: Arc<RpcClient>) -> IoHandler {
    let mut io = IoHandler::default();
    //let mut io = MetaIoHandler::with_middleware(MyMiddleware::default());
    let connection_pool =
        create_r2d2_connection_pool::<PgConnection>(DATABASE_URL.as_str(), *CONNECTION_POOL_SIZE);

    let rpc_block = RpcBlocksImpl::new(solana_client.clone(), connection_pool.clone());
    io.extend_with(rpc_block.to_delegate());
    let rpc_transaction = RpcTransactionsImpl::new(solana_client.clone(), connection_pool.clone());
    io.extend_with(rpc_transaction.to_delegate());
    let rpc_account = RpcAccountsImpl::new(solana_client.clone(), connection_pool.clone());
    io.extend_with(rpc_account.to_delegate());
    io
}
