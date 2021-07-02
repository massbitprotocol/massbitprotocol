use node_template_runtime;
use node_template_runtime::{Hash,};
use sp_runtime::DispatchError;
use support::weights::DispatchInfo;
use codec::Decode;
use serde_json;


pub type SubstrateBlock = node_template_runtime::Block;
pub type SubstrateSignedBlock = node_template_runtime::SignedBlock;
pub type SubstrateEventRecord = system::EventRecord<SystemEvent, Hash>;
pub type SubstrateExtrinsic = node_template_runtime::CheckedExtrinsic;
pub type SubstrateHeader = node_template_runtime::Header;


/// Event for the System module.
#[derive(Clone, Debug, Decode)]
pub enum SystemEvent {
    /// An extrinsic completed successfully.
    ExtrinsicSuccess(DispatchInfo),
    /// An extrinsic failed.
    ExtrinsicFailed(DispatchError, DispatchInfo),
}

trait FromPayload {
    fn from_payload(payload : Vec<u8>) -> SubstrateBlock;
}

impl FromPayload for SubstrateBlock{
    fn from_payload(payload : Vec<u8>) -> Self{
        let decode_block : Self = serde_json::from_slice(&payload).unwrap();
        decode_block
    }
}