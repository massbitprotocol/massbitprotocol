use super::orm::schema::solana_blocks::dsl as bl;
use super::orm::schema::solana_transactions::dsl as tx;
use crate::helper::parse_partially_decoded_instruction;
use crate::orm::models::SolanaTransaction;
use core::ops::Deref;
use itertools::Itertools;
use jsonrpc_core::{Params, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use massbit::prelude::serde_json::{self, json, Value};
use massbit_common::prelude::diesel::r2d2::ConnectionManager;
use massbit_common::prelude::diesel::{
    r2d2, ExpressionMethods, JoinOnDsl, PgConnection, QueryDsl, RunQueryDsl,
};
use solana_client::client_error::{ClientError, ClientErrorKind, Result as ClientResult};
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_client::rpc_request::RpcRequest;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::{Transaction, TransactionError};
use solana_transaction_status::{
    EncodedConfirmedTransaction, EncodedTransaction, UiInstruction, UiMessage, UiParsedInstruction,
    UiTransactionEncoding,
};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

#[rpc]
pub trait RpcTransactions {
    #[rpc(name = "txns_block")]
    fn get_transactions_by_block(
        &self,
        block_slot: i64,
        offset: i64,
        limit: i64,
    ) -> JsonRpcResult<serde_json::Value>;
    #[rpc(name = "txns_list")]
    fn get_transactions_list(&self, offset: i64, limit: i64) -> JsonRpcResult<serde_json::Value>;

    ///Get list transaction by address
    #[rpc(name = "txns_address")]
    fn get_transactions_by_address(
        &self,
        address: String,
        before_address: Option<String>,
        limit: usize,
    ) -> JsonRpcResult<serde_json::Value>;
    #[rpc(name = "txns_detail_db")]
    fn get_txns_detail_db(&self, tx_hash: String) -> JsonRpcResult<serde_json::Value>;
    //Get transaction detail
    #[rpc(name = "txns_detail")]
    fn get_txns_detail_chain(&self, tx_hash: String) -> JsonRpcResult<serde_json::Value>;
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
    fn get_transactions_by_block(
        &self,
        block_slot: i64,
        offset: i64,
        limit: i64,
    ) -> JsonRpcResult<serde_json::Value> {
        let headers = vec![
            "block_slot",
            "timestamp",
            "signature",
            "signers",
            "instructions",
            "fee",
            "status",
        ];
        ///Get transactions from db
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
                        Option<Vec<String>>,
                        Option<i64>,
                        Option<String>,
                    )>,
                > = tx::solana_transactions
                    .inner_join(bl::solana_blocks.on(tx::block_slot.eq(bl::block_slot)))
                    .select((
                        tx::block_slot,
                        bl::timestamp,
                        tx::signatures,
                        tx::signers,
                        tx::instructions,
                        tx::fee,
                        tx::status,
                    ))
                    .filter(tx::block_slot.eq(block_slot))
                    .order(tx::tx_index.asc())
                    .offset(offset)
                    .limit(limit)
                    .load(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    });
                txns.and_then(|values| {
                    Ok(serde_json::json!({
                        "headers": headers,
                        "values": values
                    }))
                })
            })
    }
    fn get_transactions_list(&self, offset: i64, limit: i64) -> JsonRpcResult<serde_json::Value> {
        let headers = vec![
            "block_slot",
            "timestamp",
            "signature",
            "signers",
            "instructions",
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
                        Option<Vec<String>>,
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
                        tx::instructions,
                        tx::fee,
                        tx::status,
                    ))
                    .load(conn.deref())
                    .map_err(|err| {
                        log::error!("{:?}", &err);
                        jsonrpc_core::Error::invalid_request()
                    });
                txns.and_then(|values| {
                    Ok(serde_json::json!({
                        "headers": headers,
                        "values": values
                    }))
                })
            })
    }

    fn get_transactions_by_address(
        &self,
        address: String,
        before_address: Option<String>,
        limit: usize,
    ) -> JsonRpcResult<Value> {
        let headers = vec![
            "block_slot",
            "timestamp",
            "signature",
            "signers",
            "instructions",
            "fee",
            "status",
        ];
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
                        let params = res
                            .iter()
                            .map(|tx| json!([tx.signature, UiTransactionEncoding::JsonParsed]))
                            .collect();
                        let tx_list: ClientResult<Vec<ClientResult<EncodedConfirmedTransaction>>> =
                            self.rpc_client
                                .send_batch(RpcRequest::GetTransaction, params);
                        tx_list
                            .map_err(|err| jsonrpc_core::Error::invalid_request())
                            .and_then(|txns| {
                                let vec_values: Vec<Value> = txns
                                    .iter()
                                    .filter_map(|elm| elm.as_ref().ok())
                                    .map(|elm| self.parse_encoded_confirmed_transaction_values(elm))
                                    .collect();
                                Ok(json!({
                                    "headers": headers,
                                    "values": vec_values
                                }))
                            })
                    })
            })
    }

    fn get_txns_detail_db(&self, tx_hash: String) -> JsonRpcResult<serde_json::Value> {
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

    fn get_txns_detail_chain(&self, tx_hash: String) -> JsonRpcResult<serde_json::Value> {
        log::info!("Get transaction detail for {:?}", &tx_hash);
        let start = Instant::now();
        bs58::decode(tx_hash.clone())
            .into_vec()
            .map_err(|_err| jsonrpc_core::Error::invalid_request())
            .and_then(|slide| {
                let signature = Signature::new(slide.as_slice());
                let result = self
                    .rpc_client
                    .get_transaction(&signature, UiTransactionEncoding::JsonParsed)
                    .map_err(|_err| jsonrpc_core::Error::invalid_request())
                    .and_then(|tran| {
                        // self.rpc_client.get_block(tran.slot).and_then(|block| {
                        //     for t in &block.transactions {
                        //         if let EncodedTransaction::Json(val) = &t.transaction {
                        //             if val.signatures.get(0).unwrap().clone() == tx_hash {
                        //                 println!("{:?}", t);
                        //                 break;
                        //             }
                        //         }
                        //     }
                        //     Ok(())
                        // });
                        Ok(self.parse_encoded_confirmed_transaction(&tran))
                    });
                log::info!("Get transaction from network in {:?}", start.elapsed());
                result
            })
    }
}

impl RpcTransactionsImpl {
    ///
    /// Return transaction values as a json object
    ///
    pub fn parse_encoded_confirmed_transaction(
        &self,
        tran: &EncodedConfirmedTransaction,
    ) -> serde_json::Value {
        let mut value = serde_json::json!({"slot": tran.slot, "block_time" : tran.block_time.unwrap_or_default()});
        let map = value.as_object_mut().unwrap();
        if let Some(meta) = &tran.transaction.meta {
            map.insert("fee".to_string(), json!(meta.fee));
            if meta.status.is_ok() {
                map.insert("status".to_string(), json!("1"));
            } else {
                map.insert("status".to_string(), json!("0"));
            }
            if meta.log_messages.is_some() {
                map.insert(
                    "logs".to_string(),
                    json!(meta.log_messages.as_ref().unwrap()),
                );
            }
        }
        if let EncodedTransaction::Json(transaction) = &tran.transaction.transaction {
            map.insert(
                "signature".to_string(),
                json!(transaction.signatures.get(0).unwrap_or(&String::from(""))),
            );
            match &transaction.message {
                UiMessage::Parsed(message) => {
                    map.insert(
                        "signer".to_string(),
                        json!(message
                            .account_keys
                            .get(0)
                            .and_then(|acc| Some(&acc.pubkey))
                            .unwrap_or(&String::from(""))),
                    );
                    map.insert(
                        "recent_blockhash".to_string(),
                        json!(message.recent_blockhash),
                    );
                    map.insert("instructions".to_string(), json!(message.instructions));
                }
                UiMessage::Raw(message) => {
                    map.insert(
                        "signer".to_string(),
                        json!(message
                            .account_keys
                            .get(0)
                            .and_then(|acc| Some(acc))
                            .unwrap_or(&String::from(""))),
                    );
                    map.insert(
                        "recent_blockhash".to_string(),
                        json!(message.recent_blockhash),
                    );
                    map.insert("instructions".to_string(), json!(message.instructions));
                }
            };
        }
        value
    }
    ///
    /// Return transaction values as a json array of values
    /// "block_slot",
    /// "timestamp",
    /// "signature",
    /// "signers",
    /// "instructions",
    /// "fee",
    /// "status",
    ///
    pub fn parse_encoded_confirmed_transaction_values(
        &self,
        tran: &EncodedConfirmedTransaction,
    ) -> serde_json::Value {
        let mut value = serde_json::json!([tran.slot, tran.block_time.unwrap_or_default()]);
        let arr_values = value.as_array_mut().unwrap();
        if let EncodedTransaction::Json(transaction) = &tran.transaction.transaction {
            arr_values.push(json!(transaction
                .signatures
                .get(0)
                .unwrap_or(&String::from(""))));
            match &transaction.message {
                UiMessage::Parsed(message) => {
                    arr_values.push(json!(message
                        .account_keys
                        .get(0)
                        .and_then(|acc| Some(&acc.pubkey))
                        .unwrap_or(&String::from(""))));
                    let instructions = message
                        .instructions
                        .iter()
                        .map(|inst| match inst {
                            UiInstruction::Parsed(parsed) => match parsed {
                                UiParsedInstruction::Parsed(instruction) => instruction.parsed
                                    ["type"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                UiParsedInstruction::PartiallyDecoded(instruction) => {
                                    instruction.program_id.clone()
                                    //println!("{:?}", bs58::decode(&instruction.data).into_vec().unwrap());
                                    //parse_partially_decoded_instruction(rpc_client.clone(), instruction)
                                }
                            },
                            UiInstruction::Compiled(compiled) => {
                                String::from("unknown")
                                //println!("Compiled instruction {:?}", compiled);
                            }
                        })
                        .collect::<Vec<String>>();
                    arr_values.push(json!(instructions));
                }
                UiMessage::Raw(message) => {
                    arr_values.push(json!(message
                        .account_keys
                        .get(0)
                        .and_then(|acc| Some(acc))
                        .unwrap_or(&String::from(""))));
                    let instructions = message
                        .instructions
                        .iter()
                        .filter_map(|inst| message.account_keys.get(inst.program_id_index as usize))
                        .collect::<Vec<&String>>();
                    arr_values.push(json!(instructions));
                }
            };
            if let Some(meta) = &tran.transaction.meta {
                arr_values.push(json!(meta.fee));
                if meta.status.is_ok() {
                    arr_values.push(json!("1"))
                } else {
                    arr_values.push(json!("0"))
                }
            }
        }
        value
    }
}
