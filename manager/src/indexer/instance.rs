use futures01::sync::mpsc::Sender;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

use massbit::{blockchain::DataSource, prelude::*};
use massbit::{
    blockchain::{Block, Blockchain},
    components::indexer::MappingError,
};

pub struct IndexerInstance<C: Blockchain, T: RuntimeHostBuilder<C>> {
    indexer_id: DeploymentHash,
    network: String,
    host_builder: T,

    /// Runtime hosts, one for each data source mapping.
    ///
    /// The runtime hosts are created and added in the same order the
    /// data sources appear in the subgraph manifest. Incoming block
    /// stream events are processed by the mappings in this same order.
    hosts: Vec<Arc<T::Host>>,

    /// Maps the hash of a module to a channel to the thread in which the module is instantiated.
    module_cache: HashMap<[u8; 32], Sender<T::Req>>,
}

impl<T, C: Blockchain> IndexerInstance<C, T>
where
    T: RuntimeHostBuilder<C>,
{
    pub(crate) fn from_manifest(
        logger: &Logger,
        manifest: IndexerManifest<C>,
        host_builder: T,
    ) -> Result<Self, Error> {
        let indexer_id = manifest.id.clone();
        let network = manifest.network_name();
        let templates = Arc::new(manifest.templates);

        let mut this = IndexerInstance {
            host_builder,
            indexer_id,
            network,
            hosts: Vec::new(),
            module_cache: HashMap::new(),
        };

        // Create a new runtime host for each data source in the subgraph manifest;
        // we use the same order here as in the subgraph manifest to make the
        // event processing behavior predictable
        for ds in manifest.data_sources {
            let host = this.new_host(logger.cheap_clone(), ds, templates.cheap_clone())?;
            this.hosts.push(Arc::new(host))
        }

        Ok(this)
    }

    fn new_host(
        &mut self,
        logger: Logger,
        data_source: C::DataSource,
        templates: Arc<Vec<C::DataSourceTemplate>>,
    ) -> Result<T::Host, Error> {
        let mapping_request_sender = {
            let module_bytes = data_source.runtime();
            let module_hash = tiny_keccak::keccak256(module_bytes);
            if let Some(sender) = self.module_cache.get(&module_hash) {
                sender.clone()
            } else {
                let sender =
                    T::spawn_mapping(module_bytes.to_owned(), self.indexer_id.clone(), logger)?;
                self.module_cache.insert(module_hash, sender.clone());
                sender
            }
        };
        self.host_builder.build(
            self.network.clone(),
            self.indexer_id.clone(),
            data_source,
            templates,
            mapping_request_sender,
        )
    }

    pub(crate) async fn process_trigger(
        &self,
        logger: &Logger,
        block: &Arc<C::Block>,
        trigger: &C::TriggerData,
        state: BlockState<C>,
    ) -> Result<BlockState<C>, MappingError> {
        Self::process_trigger_in_runtime_hosts(logger, &self.hosts, block, trigger, state).await
    }

    pub(crate) async fn process_trigger_in_runtime_hosts(
        logger: &Logger,
        hosts: &[Arc<T::Host>],
        block: &Arc<C::Block>,
        trigger: &C::TriggerData,
        mut state: BlockState<C>,
    ) -> Result<BlockState<C>, MappingError> {
        for host in hosts {
            let mapping_trigger =
                match host.match_and_decode(logger, trigger, block.cheap_clone())? {
                    // Trigger matches and was decoded as a mapping trigger.
                    Some(mapping_trigger) => mapping_trigger,

                    // Trigger does not match, do not process it.
                    None => continue,
                };

            state = host
                .process_mapping_trigger(logger, block.ptr(), mapping_trigger, state)
                .await?;
        }

        Ok(state)
    }

    pub(crate) fn add_dynamic_data_source(
        &mut self,
        logger: &Logger,
        data_source: C::DataSource,
        templates: Arc<Vec<C::DataSourceTemplate>>,
    ) -> Result<Option<Arc<T::Host>>, Error> {
        // `hosts` will remain ordered by the creation block.
        // See also 8f1bca33-d3b7-4035-affc-fd6161a12448.
        assert!(
            self.hosts.last().and_then(|h| h.creation_block_number())
                <= data_source.creation_block()
        );

        let host = Arc::new(self.new_host(logger.clone(), data_source, templates)?);

        Ok(if self.hosts.contains(&host) {
            None
        } else {
            self.hosts.push(host.clone());
            Some(host)
        })
    }

    pub(crate) fn revert_data_sources(&mut self, reverted_block: BlockNumber) {
        // `hosts` is ordered by the creation block.
        // See also 8f1bca33-d3b7-4035-affc-fd6161a12448.
        while self
            .hosts
            .last()
            .filter(|h| h.creation_block_number() >= Some(reverted_block))
            .is_some()
        {
            self.hosts.pop();
        }
    }
}
