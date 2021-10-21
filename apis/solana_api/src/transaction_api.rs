use super::orm::schema::solana_blocks::dsl as bl;
use super::orm::schema::solana_transactions::dsl as tx;
use crate::helper::parse_partially_decoded_instruction;
use crate::orm::models::SolanaTransaction;
use core::ops::Deref;
use jsonrpc_core::{Params, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use massbit::prelude::serde_json::{self, json, Value};
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{
    r2d2, ExpressionMethods, JoinOnDsl, PgConnection, QueryDsl, RunQueryDsl,
};
use solana_client::client_error::ClientError;
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedConfirmedTransaction, EncodedTransaction, UiInstruction, UiMessage, UiParsedInstruction,
    UiTransactionEncoding,
};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

#[rpc]
pub trait RpcTransactions {
    #[rpc(name = "txns/block")]
    fn get_block_transactions(
        &self,
        block_slot: i64,
        offset: i64,
        limit: i64,
    ) -> JsonRpcResult<serde_json::Value>;
    #[rpc(name = "txns/list")]
    fn get_list_transactions(&self, offset: i64, limit: i64) -> JsonRpcResult<serde_json::Value>;

    ///Get list transaction by address
    #[rpc(name = "txns/address")]
    fn get_address_transactions(
        &self,
        address: String,
        before_address: Option<String>,
        limit: usize,
    ) -> JsonRpcResult<serde_json::Value>;
    #[rpc(name = "txns/detail")]
    fn get_txns_detail(&self, tx_hash: String) -> JsonRpcResult<serde_json::Value>;
    #[rpc(name = "txns/detail_net")]
    fn get_txns_detail_net(&self, tx_hash: String) -> JsonRpcResult<serde_json::Value>;
}
pub struct ViewSolanaTransaction {}
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

impl RpcTransactions for RpcTransactionsImpl {
    fn get_block_transactions(
        &self,
        block_slot: i64,
        offset: i64,
        limit: i64,
    ) -> JsonRpcResult<serde_json::Value> {
        ///Get transactions from db
        let vec_transactions = self
            .connection_pool
            .get()
            .map_err(|_err| jsonrpc_core::Error::internal_error())
            .and_then(|conn| {
                let txns: JsonRpcResult<Vec<SolanaTransaction>> = tx::solana_transactions
                    .filter(tx::block_slot.eq(block_slot))
                    .order(tx::tx_index.asc())
                    .offset(offset)
                    .limit(limit)
                    .load::<SolanaTransaction>(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    });
                txns
            });
        /// Then get transactions from network
        vec_transactions.and_then(|val| Ok(serde_json::json!(val)))
    }
    fn get_list_transactions(&self, offset: i64, limit: i64) -> JsonRpcResult<serde_json::Value> {
        let headers = vec![
            "block_slot",
            "timestamp",
            "signatures",
            "signers",
            "fee",
            "status",
        ];
        self.connection_pool
            .get()
            .map_err(|_err| jsonrpc_core::Error::internal_error())
            .and_then(|conn| {
                let txns: JsonRpcResult<
                    Vec<(
                        Option<i64>,
                        Option<i64>,
                        String,
                        Option<String>,
                        Option<i64>,
                        Option<String>,
                    )>,
                > = tx::solana_transactions
                    .inner_join(bl::solana_blocks.on(tx::block_slot.eq(bl::block_slot)))
                    .order((bl::block_slot.desc(), tx::tx_index.asc()))
                    .offset(offset)
                    .limit(limit)
                    .select((
                        tx::block_slot,
                        bl::timestamp,
                        tx::signatures,
                        tx::signers,
                        tx::fee,
                        tx::status,
                    ))
                    .load(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    });
                //Todo: add instruction infos to transaction
                txns.and_then(|values| {
                    Ok(serde_json::json!({
                        "headers": headers,
                        "values": values
                    }))
                })
            })
    }

    fn get_address_transactions(
        &self,
        address: String,
        before_address: Option<String>,
        limit: usize,
    ) -> JsonRpcResult<Value> {
        Pubkey::from_str(address.as_str())
            .map_err(|_err| jsonrpc_core::Error::invalid_request())
            .and_then(|key| {
                let config = GetConfirmedSignaturesForAddress2Config {
                    before: before_address.and_then(|addr| {
                        bs58::decode(addr)
                            .into_vec()
                            .ok()
                            .and_then(|val| Some(Signature::new(val.as_slice())))
                    }),
                    until: None,
                    limit: Some(limit),
                    commitment: None,
                };
                self.rpc_client
                    .get_signatures_for_address_with_config(&key, config)
                    .map_err(|_err| jsonrpc_core::Error::invalid_request())
                    .and_then(|res| {
                        println!("{:?}", &res);
                        let params = res
                            .iter()
                            .map(|tx| json!([tx.signature, UiTransactionEncoding::JsonParsed]))
                            .collect();
                        //self.rpc_client.send_batch();
                        //Todo: get all transactions from network and parse to get instructions
                        Ok(serde_json::json!(res))
                    })
            })
    }

    fn get_txns_detail(&self, tx_hash: String) -> JsonRpcResult<serde_json::Value> {
        log::info!("Get transaction detail for {:?}", &tx_hash);
        self.connection_pool
            .get()
            .map_err(|_err| jsonrpc_core::Error::internal_error())
            .and_then(|conn| {
                let txns: JsonRpcResult<SolanaTransaction> = tx::solana_transactions
                    .filter(tx::signatures.eq(tx_hash))
                    .first::<SolanaTransaction>(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    });
                txns.and_then(|value| Ok(serde_json::json!(value)))
            })
    }

    fn get_txns_detail_net(&self, tx_hash: String) -> JsonRpcResult<serde_json::Value> {
        log::info!("Get transaction detail for {:?}", &tx_hash);
        let start = Instant::now();
        match bs58::decode(tx_hash).into_vec() {
            Ok(slide) => {
                let signature = Signature::new(slide.as_slice());
                let result = self
                    .rpc_client
                    .get_transaction(&signature, UiTransactionEncoding::JsonParsed)
                    .map_err(|err| jsonrpc_core::Error::invalid_request())
                    .and_then(|tran| {
                        Ok(parse_encoded_confirmed_transaction(
                            self.rpc_client.clone(),
                            &tran,
                        ))
                    });
                log::info!("Get transaction from network in {:?}", start.elapsed());
                result
            }
            Err(_err) => Err(jsonrpc_core::Error::invalid_request()),
        }
    }
}

fn parse_encoded_confirmed_transaction(
    rpc_client: Arc<RpcClient>,
    tran: &EncodedConfirmedTransaction,
) -> serde_json::Value {
    // tran.transaction.transaction.decode().and_then(|tran| {
    //     println!("Transaction {:?}", &tran);
    //     Some(serde_json::json!(&tran))
    // });
    if let EncodedTransaction::Json(transaction) = &tran.transaction.transaction {
        if let UiMessage::Parsed(message) = &transaction.message {
            message.instructions.iter().for_each(|inst| match inst {
                UiInstruction::Parsed(parsed) => match parsed {
                    UiParsedInstruction::Parsed(instruction) => {
                        println!("Program {:?}", instruction.program);
                    }
                    UiParsedInstruction::PartiallyDecoded(instruction) => {
                        println!("{:?}", bs58::decode(&instruction.data).into_vec().unwrap());
                        parse_partially_decoded_instruction(rpc_client.clone(), instruction)
                    }
                },
                UiInstruction::Compiled(compiled) => {
                    println!("Compiled instruction {:?}", compiled);
                }
            });
            println!("Parsed message {:?}", message);
        }
    }
    serde_json::json!(tran)
}
