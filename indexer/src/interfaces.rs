use crate::types::{EventRecord, Extrinsic, SignedBlock};

pub struct SubstrateBlock {
    pub signed_block: SignedBlock,
    pub spec_version: i64,
    pub timestamp: i64,
    pub events: Vec<EventRecord>,
}

pub struct SubstrateExtrinsic {
    pub idx: i64,
    pub extrinsic: Extrinsic,
    pub block: SubstrateBlock,
    pub events: Vec<EventRecord>,
    pub success: bool,
}

pub struct SubstrateEvent {
    pub event: EventRecord,
    pub idx: i64,
    pub extrinsic: Extrinsic,
    pub block: SubstrateBlock,
}
