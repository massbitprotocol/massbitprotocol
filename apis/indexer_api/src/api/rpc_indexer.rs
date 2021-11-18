use crate::api::rpc_types::DeployParams;
use crate::indexer_service::IndexerService;
use jsonrpc_core::{Error, Params, Response, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use massbit::prelude::serde_json::{json, Value};
use massbit::prelude::{Future, Future01CompatExt};
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{r2d2, PgConnection};
use massbit_common::prelude::r2d2::PooledConnection;
use std::sync::Arc;

#[rpc]
pub trait RpcIndexers {
    #[rpc(name = "getIndexerList")]
    fn get_indexer_list(&self, offset: i64, limit: i64) -> JsonRpcResult<Value>;
    #[rpc(name = "getIndexerDetail")]
    fn get_indexer_detail(&self, indexer_hash: String) -> JsonRpcResult<Value>;
    #[rpc(name = "getIndexerStatus")]
    fn get_indexer_status(&self, indexer_hash: String) -> JsonRpcResult<Value>;
    #[rpc(name = "deployIndexer")]
    fn deploy_indexer(&self, params: Params) -> JsonRpcResult<Value>;
}

pub struct RpcIndexersImpl {
    pub connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    pub indexer_service: IndexerService,
}
impl RpcIndexersImpl {
    pub fn new(connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        RpcIndexersImpl {
            connection_pool,
            indexer_service: IndexerService::new(),
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
impl RpcIndexers for RpcIndexersImpl {
    fn get_indexer_list(&self, offset: i64, limit: i64) -> JsonRpcResult<Value> {
        todo!()
    }

    fn get_indexer_detail(&self, indexer_hash: String) -> JsonRpcResult<Value> {
        todo!()
    }

    fn get_indexer_status(&self, indexer_hash: String) -> JsonRpcResult<Value> {
        todo!()
    }

    fn deploy_indexer(&self, params: Params) -> JsonRpcResult<Value> {
        println!("{:?}", &params);
        let deploy_params: DeployParams = params.parse().unwrap();
        println!("{:?}", &deploy_params);
        //self.indexer_service.deploy(deploy_params)
        todo!()
    }
}
