use node_template_runtime;
use node_template_runtime::Hash;
use sp_runtime::DispatchError;

pub type SignedBlock = node_template_runtime::SignedBlock;
pub type EventRecord = system::EventRecord<SystemEvent, Hash>;
pub type Extrinsic = node_template_runtime::CheckedExtrinsic;

#[derive(Clone, Debug, Decode)]
pub enum SystemEvent {
    ExtrinsicSuccess(DispatchInfo),
    ExtrinsicFailed(DispatchError, DispatchInfo),
}
