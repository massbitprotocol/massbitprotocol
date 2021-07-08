use node_template_runtime;
use std::error::Error;
use sp_runtime::traits::SignedExtension;
use serde::{Deserialize, Serialize};
use codec::{Decode, Input, WrapperTypeDecode};


// Main data type for substrate indexing
pub type SubstrateBlock = node_template_runtime::Block;
pub type SubstrateEventRecord = system::EventRecord<node_template_runtime::Event, node_template_runtime::Hash>;
pub type SubstrateUncheckedExtrinsic = node_template_runtime::UncheckedExtrinsic;

// Not use for indexing yet
pub type SubstrateHeader = node_template_runtime::Header;
pub type SubstrateCheckedExtrinsic = node_template_runtime::CheckedExtrinsic;
pub type SubstrateSignedBlock = node_template_runtime::SignedBlock;

pub fn decode<T>(payload: &mut Vec<u8>) -> Result<T, Box<dyn Error>>
    where T: Decode/* + Into<Vec<u8>>*/,
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

