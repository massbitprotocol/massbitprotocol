use codec::{Decode, Encode};
use node_template_runtime;
use std::error::Error;

type Number = u32;
type Date = i64;
type Event = system::EventRecord<node_template_runtime::Event, node_template_runtime::Hash>;
type Extrinsic = node_template_runtime::UncheckedExtrinsic;
type Block = node_template_runtime::Block;
type Hash = node_template_runtime::Hash;

// Similar to
// https://github.com/subquery/subql/blob/93afc96d7ee0ff56d4dd62d8a145088f5bb5e3ec/packages/types/src/interfaces.ts#L18
#[derive(PartialEq, Eq, Clone, Encode, Decode, Debug)]
pub struct SubstrateBlock {
    pub version: String,
    pub timestamp: Date,
    pub block: Block,
    pub events: Vec<Event>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Debug)]
pub struct SubstrateUncheckedExtrinsic {
    pub block_number: Number,
    pub extrinsic: Extrinsic,
    pub block: SubstrateBlock,
    pub events: Vec<SubstrateEventRecord>,
    pub success: bool,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Debug)]
pub struct SubstrateEventRecord {
    //block_number: Number,
    pub event: Event,
    //extrinsic: Option<Box<ExtExtrinsic>>,
    //block: Box<SubstrateBlock>,
}

// Not use for indexing yet
pub type SubstrateHeader = node_template_runtime::Header;
pub type SubstrateCheckedExtrinsic = node_template_runtime::CheckedExtrinsic;
pub type SubstrateSignedBlock = node_template_runtime::SignedBlock;

pub trait ExtrinsicTrait {
    fn is_signed(&self) -> bool;
    fn get_hash(&self) -> Hash;
}

// impl ExtrinsicTrait for ExtExtrinsic {
//     fn is_signed(&self) -> bool {
//         self.extrinsic.is_signed();
//     }
//     fn get_hash(&self) -> Hash {
//         self.extrinsic.get_hash();
//     }
// }

pub fn get_extrinsics_from_block(block: &SubstrateBlock) -> Vec<SubstrateUncheckedExtrinsic> {
    let iter = block.block.extrinsics.iter();
    let extrinsics = iter
        .map(|extrinsic| {
            //let hash = extrinsic.get_hash();
            SubstrateUncheckedExtrinsic {
                block_number: block.block.header.number,
                extrinsic: (*extrinsic).clone(),
                block: block.clone(),
                // Todo: add event of this extrinsic
                events: Vec::new(),
                // Todo: Check events to know the extrinsic is success
                // https://github.com/subquery/subql/blob/bec4047dccac213692a0186d55383e5be5c5c2aa/packages/node/src/utils/substrate.ts#L70
                success: true,
            }
        })
        .collect();
    extrinsics
}

pub fn decode<T>(payload: &mut Vec<u8>) -> Result<T, Box<dyn Error>>
where
    T: Decode,
{
    Ok(Decode::decode(&mut payload.as_slice()).unwrap())
}

pub fn decode_transactions(
    payload: &mut Vec<u8>,
) -> Result<Vec<SubstrateUncheckedExtrinsic>, Box<dyn Error>> {
    let transactions: Vec<Vec<u8>> = Decode::decode(&mut payload.as_slice()).unwrap();
    println!("transactions: {:?}", transactions);

    Ok(transactions
        .into_iter()
        .map(|encode| Decode::decode(&mut encode.as_slice()).unwrap())
        .collect())
}
