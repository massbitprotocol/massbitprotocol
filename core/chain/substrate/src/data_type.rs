use node_template_runtime;
use std::error::Error;
use sp_runtime::traits::SignedExtension;
use serde::{Deserialize, Serialize};
use codec::{Encode, Decode, Input, WrapperTypeDecode};



//********************** SUBSTRATE ********************************
// Main data type for substrate indexing
pub type SubstrateBlock = ExtBlock;
pub type SubstrateUncheckedExtrinsic = ExtExtrinsic;
pub type SubstrateEventRecord = ExtEvent;

type Number = u64;
type Date = u16;
type Event = system::EventRecord<node_template_runtime::Event, node_template_runtime::Hash>;
type Extrinsic = node_template_runtime::UncheckedExtrinsic;

#[derive(PartialEq, Eq, Clone, Encode, Decode, Debug)]
struct ExtBlock {
    version: String,
    timestamp: Date,
    block: node_template_runtime::Block,
    events: Event,
}


struct ExtExtrinsic {
    block_number: Number,
    extrinsic: Extrinsic,
    block: Box<ExtBlock>,
    events: Event,
    success: bool,
}

struct ExtEvent {
    block_number: Number,
    event: Event,
    extrinsic: Option<Box<ExtExtrinsic>>,
    block: Box<SubstrateBlock>,
}

// Not use for indexing yet
pub type SubstrateHeader = node_template_runtime::Header;
pub type SubstrateCheckedExtrinsic = node_template_runtime::CheckedExtrinsic;
pub type SubstrateSignedBlock = node_template_runtime::SignedBlock;



pub fn decode<T>(payload: &mut Vec<u8>) -> Result<T, Box<dyn Error>>
    where T: Decode,
{
    Ok(Decode::decode(&mut payload.as_slice()).unwrap())
}

pub fn decode_transactions(payload: &mut  Vec<u8>) -> Result<Vec<SubstrateUncheckedExtrinsic>, Box<dyn Error>>{
    let mut transactions: Vec<Vec<u8>> = Decode::decode(&mut payload.as_slice()).unwrap();
    println!("transactions: {:?}", transactions);

    Ok(transactions
        .into_iter()
        .map(|encode| Decode::decode(&mut encode.as_slice()).unwrap())
        .collect())
}


