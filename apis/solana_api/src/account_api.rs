//use crate::orm::schema::solana_inst_assigns::columns::account;

use diesel::r2d2::PooledConnection;
use jsonrpc_core::Result as JsonRpcResult;
use jsonrpc_derive::rpc;
use massbit::prelude::{
    serde::Serialize,
    serde_json::{json, Value},
};

use massbit_chain_solana::data_type::TOKEN_PROGRAM_ID;
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{r2d2, PgConnection};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;

use std::sync::Arc;
use tokio::time::Instant;

#[rpc]
pub trait RpcAccounts {
    #[rpc(name = "getAccountInfo")]
    fn get_account_data(&self, pubkey: String, encode_type: String) -> JsonRpcResult<Value>;
}

pub struct RpcAccountsImpl {
    pub rpc_client: Arc<RpcClient>,
    pub connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}
impl RpcAccountsImpl {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        RpcAccountsImpl {
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

#[derive(Debug, Serialize)]
pub enum AccountType {
    Unidentified,
    Invalid,
    ProgramAccount,
    Account,
    TokenAccount,
    MintAccount,
}

fn get_account_type(res_account: &Value) -> AccountType {
    println!("res_account: {:?}", res_account);

    if let Some(value) = res_account.get("value") {
        if value["executable"] == true {
            return AccountType::ProgramAccount;
        } else {
            if value["owner"] == TOKEN_PROGRAM_ID {
                if value["data"]["parsed"]["type"] == "mint" {
                    return AccountType::MintAccount;
                }
                if value["data"]["parsed"]["type"] == "account" {
                    return AccountType::TokenAccount;
                }
                return AccountType::Unidentified;
            }
        }
        return AccountType::Account;
    }
    AccountType::Invalid
}

impl RpcAccounts for RpcAccountsImpl {
    fn get_account_data(&self, pubkey: String, encode_type: String) -> JsonRpcResult<Value> {
        log::info!("Get account detail for {:?}", &pubkey);
        let _start = Instant::now();
        let params = json!([pubkey, { "encoding": encode_type }]);

        println!("params: {:?}", params);
        let mut res: JsonRpcResult<Value> = self
            .rpc_client
            .send(RpcRequest::GetAccountInfo, params)
            .map_err(|err| jsonrpc_core::Error::invalid_params(format!("{:?}", err)));

        if let Ok(ref mut res) = res {
            let account_type = get_account_type(res);
            println!("account_type: {:?}", &account_type);
            res["accountType"] = json!(account_type);
        }

        println!("res: {:?}", &res);

        res
    }
}
