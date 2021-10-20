use crate::schema::*;
use bigdecimal::{BigDecimal, FromPrimitive};
use massbit::components::ethereum::LightEthereumBlock;
use massbit::prelude::web3::types::Transaction;

//https://kotiri.com/2018/01/31/postgresql-diesel-rust-types.html
#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "ethereum_blocks"]
pub struct EthereumBlock {
    pub block_hash: String,
    pub block_number: Option<i64>,
    pub transaction_number: i64,
    pub timestamp: i64,
    pub validated_by: Option<String>,
    pub reward: Option<BigDecimal>,
    pub difficulty: Option<BigDecimal>,
    pub total_difficulty: Option<BigDecimal>,
    pub size: Option<i64>,
    pub gas_used: Option<BigDecimal>,
    pub gas_limit: Option<BigDecimal>,
    pub extra_data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "ethereum_transactions"]
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
    pub timestamp: i64,
}

impl From<&LightEthereumBlock> for EthereumBlock {
    fn from(block: &LightEthereumBlock) -> Self {
        let block_hash = match block.hash {
            Some(hash) => format!("0x{}", hex::encode(hash.as_bytes()).trim_start_matches('0')),
            None => String::from(""),
        };
        let block_number = match block.number {
            None => None,
            Some(val) => Some(val.as_u64() as i64),
        };
        let timestamp = block.timestamp.as_u64() as i64;
        let _validator = format!(
            "0x{}",
            hex::encode(block.author.as_bytes()).trim_start_matches('0')
        );
        let validator = Some(format!(
            "0x{}",
            hex::encode(block.author.as_bytes()).trim_start_matches('0')
        ));
        // let validator = match block.author {
        //     Some(author) => Some(format!(
        //         "0x{}",
        //         hex::encode(block.author.as_bytes()).trim_start_matches('0')
        //     )),
        //     None => None
        // };
        let total_difficulty = match block.total_difficulty {
            None => None,
            Some(val) => BigDecimal::from_u128(val.as_u128()),
        };
        let size = match block.size {
            None => None,
            Some(val) => Some(val.as_u64() as i64),
        };
        let gas_limit = BigDecimal::from_u128(block.gas_limit.as_u128());
        let gas_used = BigDecimal::from_u128(block.gas_used.as_u128());
        EthereumBlock {
            block_hash,
            block_number,
            transaction_number: block.transactions.len() as i64,
            timestamp,
            validated_by: validator,
            reward: None,
            difficulty: BigDecimal::from_u128(block.difficulty.as_u128()),
            total_difficulty,
            size,
            gas_used,
            gas_limit,
            extra_data: Some(block.extra_data.0.clone()),
        }
    }
}
impl From<&Transaction> for EthereumTransaction {
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
            _ => None,
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
            _ => None,
        };
        let value = match BigDecimal::from_u128(trans.value.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val,
        };
        let gas_limit = match BigDecimal::from_u128(trans.gas.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val,
        };
        let gas_price = match BigDecimal::from_u128(trans.gas_price.as_u128()) {
            None => BigDecimal::from(0),
            Some(val) => val,
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
