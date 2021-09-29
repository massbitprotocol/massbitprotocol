use massbit_common::prelude::bigdecimal::{BigDecimal, FromPrimitive};
use massbit_common::prelude::tokio_postgres::Transaction;

#[derive(Debug, Clone)]
pub struct EthereumTransaction {
    pub transaction_hash: String,
    pub block_hash: Option<String>,
    pub block_number: Option<i64>,
    pub nonce: Option<BigDecimal>,
    pub sender: String,
    pub receiver: Option<String>,
    pub value: BigDecimal,
    pub gas_limit: BigDecimal,
    pub gas_price: BigDecimal,
    pub timestamp: i64
}

impl  From<&Transaction> for EthereumTransaction {
    fn from(trans: &Transaction) -> Self {
        let transaction_hash = format!(
            "0x{}",
            hex::encode(trans.hash.as_bytes()).trim_start_matches('0')
        );
        let block_hash = match trans.block_hash {
            None => None,
            Some(hash) => Some(format!(
                "0x{}",
                hex::encode(hash.as_bytes()).trim_start_matches('0')
            )),
        };
        let block_number = match trans.block_number {
            Some(val) => Some(val.as_u64() as i64),
            _ => None
        };
        let sender = format!(
            "0x{}",
            hex::encode(trans.from.as_bytes()).trim_start_matches('0')
        );
        let receiver = match trans.to {
            Some(val) => Some(format!(
                "0x{}",
                hex::encode(val.as_bytes()).trim_start_matches('0')
            )),
            _ => None
        };
        let value = match BigDecimal::from_u128(trans.value.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val
        };
        let gas_limit = match BigDecimal::from_u128(trans.gas.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val
        };
        let gas_price = match BigDecimal::from_u128(trans.gas_price.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val
        };
        EthereumTransaction {
            transaction_hash,
            block_hash,
            block_number,
            nonce: BigDecimal::from_u128(trans.value.as_u128()),
            sender,
            receiver,
            value,
            gas_limit,
            gas_price,
            timestamp: 0,
        }
    }
}