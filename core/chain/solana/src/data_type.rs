//********************** SOLANA ********************************
use solana_transaction_status;
use std::error::Error;
use serde_json;
use serde::{Deserialize, Serialize};

// EncodedConfirmedBlock is block with vec of EncodedTransactionWithStatusMeta.
pub type SolanaBlock = solana_transaction_status::EncodedConfirmedBlock;
pub type SolanaTransaction = solana_transaction_status::EncodedTransactionWithStatusMeta;
// The most similar Event concept in Solana is log_messages in UiTransactionStatusMeta in EncodedTransactionWithStatusMeta
pub type SolanaLogMessages = Option<Vec<String>>;

pub fn decode(payload: &mut Vec<u8>) -> Result<SolanaBlock, Box<dyn Error>>
{
    let decode_block: SolanaBlock = serde_json::from_slice(&payload).unwrap();
    Ok(decode_block)
}

type Number = u32;
type Date = u16;
type LogMessages = Option<Vec<String>>;
type Transaction = solana_transaction_status::EncodedTransactionWithStatusMeta;
type Block = solana_transaction_status::EncodedConfirmedBlock;
type Hash = solana_transaction_status::Hash;


// Similar to
// https://github.com/subquery/subql/blob/93afc96d7ee0ff56d4dd62d8a145088f5bb5e3ec/packages/types/src/interfaces.ts#L18
#[derive(PartialEq, Eq, Clone, Encode, Decode, Debug)]
pub struct ExtBlock {
    pub version: String,
    pub timestamp: Date,
    pub block: Block,
    pub events: Vec<Event>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Debug)]
pub struct ExtTransaction {
    block_number: Number,
    transaction: Transaction,
    block: ExtBlock,
    events: Vec<ExtEvent>,
    success: bool,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Debug)]
pub struct ExtEvent {
    //block_number: Number,
    pub event: Event,
    //extrinsic: Option<Box<ExtExtrinsic>>,
    //block: Box<SubstrateBlock>,
}