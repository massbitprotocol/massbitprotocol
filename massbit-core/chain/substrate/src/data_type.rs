use node_template_runtime;
use node_template_runtime::Hash;
use sp_runtime::DispatchError;
use support::weights::DispatchInfo;
use codec::Decode;


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





