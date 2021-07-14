//********************** SOLANA ********************************
use solana_transaction_status;
use std::error::Error;
use serde_json;
use serde::{Deserialize, Serialize};

// EncodedConfirmedBlock is block with vec of EncodedTransactionWithStatusMeta.
pub type SolanaBlock = solana_transaction_status::EncodedConfirmedBlock;
pub type SolanaTransaction = solana_transaction_status::EncodedTransactionWithStatusMeta;
// The most similar Event concept in Solana is log_messages in UiTransactionStatusMeta in EncodedTransactionWithStatusMeta
pub type SolanaEvent = Option<Vec<String>>;

pub fn decode(payload: &mut Vec<u8>) -> Result<SolanaBlock, Box<dyn Error>>
{
    let decode_block: SolanaBlock = serde_json::from_slice(&payload).unwrap();
    Ok(decode_block)
}
