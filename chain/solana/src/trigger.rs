use crate::chain::Chain;
use crate::data_source::DataSource;
use massbit::blockchain as bc;
use massbit::blockchain::TriggerData;
use massbit::prelude::prost::alloc::fmt::Formatter;
use massbit::prelude::*;
use massbit::runtime::{AscHeap, AscPtr, DeterministicHostError};
use std::cmp::Ordering;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TriggerFilter {
    addresses: Vec<String>,
}

impl bc::TriggerFilter<Chain> for TriggerFilter {
    fn from_data_sources<'a>(data_sources: impl Iterator<Item = &'a DataSource> + Clone) -> Self {
        let mut this = Self::default();
        this.extend(data_sources);
        this
    }

    fn extend<'a>(&mut self, data_sources: impl Iterator<Item = &'a DataSource> + Clone) {
        data_sources.for_each(|ds| {
            if let Some(addr) = &ds.source.address {
                self.addresses.push(addr.clone());
            }
        });
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolanaTriggerData {}

impl Ord for SolanaTriggerData {
    fn cmp(&self, other: &Self) -> Ordering {
        todo!()
    }
}

impl Eq for SolanaTriggerData {}

impl PartialEq<Self> for SolanaTriggerData {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl PartialOrd for SolanaTriggerData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TriggerData for SolanaTriggerData {
    fn error_context(&self) -> String {
        todo!()
    }
}
pub struct SolanaMappingTrigger {}
impl std::fmt::Debug for SolanaMappingTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> prost::alloc::fmt::Result {
        todo!()
    }
}

impl bc::MappingTrigger for SolanaMappingTrigger {
    fn handler_name(&self) -> &str {
        todo!()
    }

    fn to_asc_ptr<H: AscHeap>(self, heap: &mut H) -> Result<AscPtr<()>, DeterministicHostError> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolanaBlockTriggerType {
    Every,
}
