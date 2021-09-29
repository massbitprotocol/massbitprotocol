use crate::schema::*;
use bigdecimal::{BigDecimal, FromPrimitive};
use massbit_chain_ethereum::data_type::{LightEthereumBlock};
use graph::prelude::web3::types::Transaction;

//https://kotiri.com/2018/01/31/postgresql-diesel-rust-types.html
#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "ethereum_block"]
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

impl From<&LightEthereumBlock> for  EthereumBlock {
    fn from(block: &LightEthereumBlock) -> Self {
        let block_hash = match block.hash {
            Some(hash) => format!(
                "0x{}",
                hex::encode(hash.as_bytes()).trim_start_matches('0')
            ),
            None => String::from("")
        };
        let block_number = match block.number {
            None => None,
            Some(val) => Some(val.as_u64() as i64)
        };
        let timestamp= block.timestamp.as_u64() as i64;
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
            Some(val) => Some(val.as_u64() as i64)
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
            extra_data: Some(block.extra_data.0.clone())
        }
    }
}