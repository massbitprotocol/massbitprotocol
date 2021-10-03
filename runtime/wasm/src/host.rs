use std::cmp::PartialEq;
use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use futures::sync::mpsc::Sender;
use futures03::channel::oneshot::channel;
use massbit::blockchain::HostFn;
use massbit::blockchain::RuntimeAdapter;
use massbit::blockchain::{Blockchain, DataSource, MappingTrigger as _};
use massbit::components::indexer::{BlockState, MappingError};
use massbit::components::link_resolver::LinkResolver;
use massbit::components::store::IndexerStore;
use massbit::prelude::{
    RuntimeHost as RuntimeHostTrait, RuntimeHostBuilder as RuntimeHostBuilderTrait, *,
};

use crate::mapping::{MappingContext, MappingRequest};
use crate::{host_exports::HostExports, module::ExperimentalFeatures};

lazy_static! {
    static ref TIMEOUT: Option<Duration> = std::env::var("GRAPH_MAPPING_HANDLER_TIMEOUT")
        .ok()
        .map(|s| u64::from_str(&s).expect("Invalid value for GRAPH_MAPPING_HANDLER_TIMEOUT"))
        .map(Duration::from_secs);
    static ref ALLOW_NON_DETERMINISTIC_IPFS: bool =
        std::env::var("GRAPH_ALLOW_NON_DETERMINISTIC_IPFS").is_ok();
}

pub struct RuntimeHostBuilder<C: Blockchain> {
    runtime_adapter: Arc<C::RuntimeAdapter>,
    link_resolver: Arc<dyn LinkResolver>,
    store: Arc<dyn IndexerStore>,
}

impl<C: Blockchain> Clone for RuntimeHostBuilder<C> {
    fn clone(&self) -> Self {
        RuntimeHostBuilder {
            runtime_adapter: self.runtime_adapter.cheap_clone(),
            link_resolver: self.link_resolver.cheap_clone(),
            store: self.store.cheap_clone(),
        }
    }
}

impl<C: Blockchain> RuntimeHostBuilder<C> {
    pub fn new(
        runtime_adapter: Arc<C::RuntimeAdapter>,
        link_resolver: Arc<dyn LinkResolver>,
        store: Arc<dyn IndexerStore>,
    ) -> Self {
        RuntimeHostBuilder {
            runtime_adapter,
            link_resolver,
            store,
        }
    }
}

impl<C: Blockchain> RuntimeHostBuilderTrait<C> for RuntimeHostBuilder<C> {
    type Host = RuntimeHost<C>;
    type Req = MappingRequest<C>;

    fn spawn_mapping(
        raw_module: Vec<u8>,
        subgraph_id: DeploymentHash,
        logger: Logger,
    ) -> Result<Sender<Self::Req>, Error> {
        let experimental_features = ExperimentalFeatures {
            allow_non_deterministic_ipfs: *ALLOW_NON_DETERMINISTIC_IPFS,
        };
        crate::mapping::spawn_module(
            raw_module,
            subgraph_id,
            tokio::runtime::Handle::current(),
            *TIMEOUT,
            experimental_features,
            logger,
        )
    }

    fn build(
        &self,
        network_name: String,
        subgraph_id: DeploymentHash,
        data_source: C::DataSource,
        templates: Arc<Vec<C::DataSourceTemplate>>,
        mapping_request_sender: Sender<MappingRequest<C>>,
    ) -> Result<Self::Host, Error> {
        RuntimeHost::new(
            self.runtime_adapter.cheap_clone(),
            self.link_resolver.clone(),
            network_name,
            subgraph_id,
            data_source,
            templates,
            mapping_request_sender,
        )
    }
}

pub struct RuntimeHost<C: Blockchain> {
    host_fns: Arc<Vec<HostFn>>,
    data_source: C::DataSource,
    mapping_request_sender: Sender<MappingRequest<C>>,
    host_exports: Arc<HostExports<C>>,
}

impl<C> RuntimeHost<C>
where
    C: Blockchain,
{
    fn new(
        runtime_adapter: Arc<C::RuntimeAdapter>,
        link_resolver: Arc<dyn LinkResolver>,
        network_name: String,
        subgraph_id: DeploymentHash,
        data_source: C::DataSource,
        templates: Arc<Vec<C::DataSourceTemplate>>,
        mapping_request_sender: Sender<MappingRequest<C>>,
    ) -> Result<Self, Error> {
        // Create new instance of externally hosted functions invoker. The `Arc` is simply to avoid
        // implementing `Clone` for `HostExports`.
        let host_exports = Arc::new(HostExports::new(
            subgraph_id,
            &data_source,
            network_name,
            templates,
            link_resolver,
        ));

        let host_fns = Arc::new(runtime_adapter.host_fns(&data_source)?);

        Ok(RuntimeHost {
            host_fns,
            data_source,
            mapping_request_sender,
            host_exports,
        })
    }

    /// Sends a MappingRequest to the thread which owns the host,
    /// and awaits the result.
    async fn send_mapping_request(
        &self,
        logger: &Logger,
        state: BlockState<C>,
        trigger: C::MappingTrigger,
        block_ptr: BlockPtr,
    ) -> Result<BlockState<C>, MappingError> {
        let handler = trigger.handler_name().to_string();
        let extras = trigger.logging_extras();

        let (result_sender, result_receiver) = channel();

        self.mapping_request_sender
            .clone()
            .send(MappingRequest {
                ctx: MappingContext {
                    logger: logger.cheap_clone(),
                    state,
                    host_exports: self.host_exports.cheap_clone(),
                    block_ptr,
                    host_fns: self.host_fns.cheap_clone(),
                },
                trigger,
                result_sender,
            })
            .compat()
            .await
            .context("Mapping terminated before passing in trigger")?;

        let result = result_receiver
            .await
            .context("Mapping terminated before handling trigger")?;

        info!(
            logger, "Done processing trigger";
            &extras,
            "handler" => handler,
            "data_source" => &self.data_source.name(),
        );

        result
    }
}

#[async_trait]
impl<C: Blockchain> RuntimeHostTrait<C> for RuntimeHost<C> {
    fn match_and_decode(
        &self,
        logger: &Logger,
        trigger: &C::TriggerData,
        block: Arc<C::Block>,
    ) -> Result<Option<C::MappingTrigger>, Error> {
        self.data_source.match_and_decode(logger, trigger, block)
    }

    async fn process_mapping_trigger(
        &self,
        logger: &Logger,
        block_ptr: BlockPtr,
        trigger: C::MappingTrigger,
        state: BlockState<C>,
    ) -> Result<BlockState<C>, MappingError> {
        self.send_mapping_request(logger, state, trigger, block_ptr)
            .await
    }

    fn creation_block_number(&self) -> Option<BlockNumber> {
        self.data_source.creation_block()
    }
}

impl<C: Blockchain> PartialEq for RuntimeHost<C> {
    fn eq(&self, other: &Self) -> bool {
        self.data_source.is_duplicate_of(&other.data_source)
    }
}
