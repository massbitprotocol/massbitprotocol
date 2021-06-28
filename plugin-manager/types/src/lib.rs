// use node_template_runtime;
// use node_template_runtime::Hash;
// use sp_runtime::DispatchError;
//
// pub type SignedBlock = node_template_runtime::SignedBlock;
// pub type EventRecord = system::EventRecord<SystemEvent, Hash>;
// pub type Extrinsic = node_template_runtime::CheckedExtrinsic;
//
// #[derive(Clone, Debug, Decode)]
// pub enum SystemEvent {
//     ExtrinsicSuccess(DispatchInfo),
//     ExtrinsicFailed(DispatchError, DispatchInfo),
// }
//
// pub struct SubstrateBlock {
//     pub signed_block: SignedBlock,
//     pub spec_version: i64,
//     pub timestamp: i64,
//     pub events: Vec<EventRecord>,
// }
//
// pub struct SubstrateExtrinsic {
//     pub idx: i64,
//     pub extrinsic: Extrinsic,
//     pub block: SubstrateBlock,
//     pub events: Vec<EventRecord>,
//     pub success: bool,
// }
//
// pub struct SubstrateEvent {
//     pub event: EventRecord,
//     pub idx: i64,
//     pub extrinsic: Extrinsic,
//     pub block: SubstrateBlock,
// }

pub struct SubstrateBlock {
    pub idx: i64,
}

pub struct SubstrateExtrinsic {
    pub idx: i64,
}

pub struct SubstrateEvent {
    pub idx: i64,
}
