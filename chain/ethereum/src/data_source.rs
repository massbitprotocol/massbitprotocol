use tiny_keccak::Keccak;
use web3::types::H256;

use massbit::components::link_resolver::LinkResolver;
use massbit::data::indexer::Source;
use massbit::{blockchain, prelude::*};

use crate::chain::Chain;

/// Runtime representation of a data source.
// Note: Not great for memory usage that this needs to be `Clone`, considering how there may be tens
// of thousands of data sources in memory at once.
#[derive(Clone, Debug)]
pub struct DataSource {
    pub kind: String,
    pub network: Option<String>,
    pub name: String,
    pub source: Source,
    pub mapping: Mapping,
    pub creation_block: Option<BlockNumber>,
}

impl blockchain::DataSource<Chain> for DataSource {
    fn start_block(&self) -> BlockNumber {
        self.source.start_block
    }
}

impl DataSource {
    fn from_manifest(
        kind: String,
        network: Option<String>,
        name: String,
        source: Source,
        mapping: Mapping,
    ) -> Result<Self, Error> {
        // Data sources in the manifest are created "before genesis" so they have no creation block.
        let creation_block = None;
        Ok(DataSource {
            kind,
            network,
            name,
            source,
            mapping,
            creation_block,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct UnresolvedDataSource {
    pub kind: String,
    pub network: Option<String>,
    pub name: String,
    pub source: Source,
    pub mapping: UnresolvedMapping,
}

#[async_trait]
impl blockchain::UnresolvedDataSource<Chain> for UnresolvedDataSource {
    async fn resolve(self, resolver: &impl LinkResolver) -> Result<DataSource, Error> {
        let UnresolvedDataSource {
            kind,
            network,
            name,
            source,
            mapping,
        } = self;

        info!("Resolve data source, name: {}", &name);

        let mapping = mapping.resolve(&*resolver).await?;
        DataSource::from_manifest(kind, network, name, source, mapping)
    }
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnresolvedMapping {
    #[serde(default)]
    pub block_handlers: Vec<MappingBlockHandler>,
    #[serde(default)]
    pub call_handlers: Vec<MappingCallHandler>,
    #[serde(default)]
    pub event_handlers: Vec<MappingEventHandler>,
}

#[derive(Clone, Debug)]
pub struct Mapping {
    pub block_handlers: Vec<MappingBlockHandler>,
    pub call_handlers: Vec<MappingCallHandler>,
    pub event_handlers: Vec<MappingEventHandler>,
}

impl UnresolvedMapping {
    pub async fn resolve(self, resolver: &impl LinkResolver) -> Result<Mapping, anyhow::Error> {
        let UnresolvedMapping {
            block_handlers,
            call_handlers,
            event_handlers,
        } = self;

        Ok(Mapping {
            block_handlers: block_handlers.clone(),
            call_handlers: call_handlers.clone(),
            event_handlers: event_handlers.clone(),
        })
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingBlockHandler {
    pub handler: String,
    pub filter: Option<BlockHandlerFilter>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BlockHandlerFilter {
    // Call filter will trigger on all blocks where the data source contract
    // address has been called
    Call,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingCallHandler {
    pub function: String,
    pub handler: String,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingEventHandler {
    pub event: String,
    pub topic0: Option<H256>,
    pub handler: String,
}

impl MappingEventHandler {
    pub fn topic0(&self) -> H256 {
        self.topic0
            .unwrap_or_else(|| string_to_h256(&self.event.replace("indexed ", "")))
    }
}

/// Hashes a string to a H256 hash.
fn string_to_h256(s: &str) -> H256 {
    let mut result = [0u8; 32];
    let data = s.replace(" ", "").into_bytes();
    let mut sponge = Keccak::new_keccak256();
    sponge.update(&data);
    sponge.finalize(&mut result);

    // This was deprecated but the replacement seems to not be available in the
    // version web3 uses.
    #[allow(deprecated)]
    H256::from_slice(&result)
}
