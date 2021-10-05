use std::cmp::PartialEq;
use std::sync::Arc;

use anyhow::Error;
use async_trait::async_trait;
use futures::sync::mpsc;

use crate::blockchain::Blockchain;
use crate::components::indexer::instance::BlockState;
use crate::prelude::*;
use crate::runtime::DeterministicHostError;

#[derive(Debug)]
pub enum MappingError {
    /// A possible reorg was detected while running the mapping.
    PossibleReorg(anyhow::Error),
    Unknown(anyhow::Error),
}

impl From<anyhow::Error> for MappingError {
    fn from(e: anyhow::Error) -> Self {
        MappingError::Unknown(e)
    }
}

impl From<DeterministicHostError> for MappingError {
    fn from(value: DeterministicHostError) -> MappingError {
        MappingError::Unknown(value.0)
    }
}

impl MappingError {
    pub fn context(self, s: String) -> Self {
        use MappingError::*;
        match self {
            PossibleReorg(e) => PossibleReorg(e.context(s)),
            Unknown(e) => Unknown(e.context(s)),
        }
    }
}

/// Common trait for runtime host implementations.
#[async_trait]
pub trait RuntimeHost<C: Blockchain>: Send + Sync + 'static {
    fn match_and_decode(
        &self,
        logger: &Logger,
        trigger: &C::TriggerData,
        block: Arc<C::Block>,
    ) -> Result<Option<C::MappingTrigger>, Error>;

    async fn process_mapping_trigger(
        &self,
        logger: &Logger,
        block_ptr: BlockPtr,
        trigger: C::MappingTrigger,
        state: BlockState<C>,
    ) -> Result<BlockState<C>, MappingError>;

    /// Block number in which this host was created.
    /// Returns `None` for static data sources.
    fn creation_block_number(&self) -> Option<BlockNumber>;
}

pub trait RuntimeHostBuilder<C: Blockchain>: Clone + Send + Sync + 'static {
    type Host: RuntimeHost<C> + PartialEq;
    type Req: 'static + Send;

    /// Build a new runtime host for a indexer data source.
    fn build(
        &self,
        network_name: String,
        indexer_id: DeploymentHash,
        data_source: C::DataSource,
        top_level_templates: Arc<Vec<C::DataSourceTemplate>>,
        mapping_request_sender: mpsc::Sender<Self::Req>,
    ) -> Result<Self::Host, Error>;

    /// Spawn a mapping and return a channel for mapping requests. The sender should be able to be
    /// cached and shared among mappings that use the same wasm file.
    fn spawn_mapping(
        raw_module: Vec<u8>,
        subgraph_id: DeploymentHash,
        logger: Logger,
    ) -> Result<mpsc::Sender<Self::Req>, anyhow::Error>;
}
