use crate::core::{AdapterError, MessageHandler};
pub use crate::stream_mod::{DataType, GenericDataProto};
use crate::EthereumWasmHandlerProxy;
use graph::blockchain::types::{BlockHash, BlockPtr};
use graph::blockchain::{Blockchain, DataSource as DataSourceTrait, HostFn};
use graph::cheap_clone::CheapClone;
use graph::components::metrics::stopwatch::StopwatchMetrics;
use graph::components::store::{ModificationsAndCache, StoreError, WritableStore};
use graph::components::subgraph::{BlockState, HostMetrics};
use graph::data::subgraph::DeploymentHash;
use graph::log::logger;
use graph_chain_ethereum::trigger::EthereumBlockTriggerType;
use graph_chain_ethereum::{
    chain::BlockFinality, trigger::EthereumTrigger, Chain, DataSource, DataSourceTemplate
};
use graph_mock::MockMetricsRegistry;
use graph_runtime_wasm::ValidModule;
use index_store::postgres::store_builder::*;
use index_store::Store;
use libloading::Library;
use massbit_chain_ethereum::data_type::{
    decode, EthereumBlock, EthereumEvent, EthereumTransaction,
};
use massbit_common::prelude::anyhow;
use massbit_runtime_wasm::host_exports::create_ethereum_call;
use massbit_runtime_wasm::prelude::Logger;
use massbit_runtime_wasm::{slog, HostExports, MappingContext, WasmInstance};
use std::convert::TryFrom;
use std::time::Instant;
use std::{error::Error, sync::Arc};

crate::prepare_adapter!(Ethereum, {
    handle_block: EthereumBlock,
    handle_transaction: EthereumTransaction,
    handle_event: EthereumEvent
});
impl MessageHandler for EthereumWasmHandlerProxy {
    fn handle_wasm_mapping(&mut self, data: &mut GenericDataProto) -> Result<(), Box<dyn Error>> {
        log::info!("{} call handle_wasm_mapping", &*COMPONENT_NAME);
        let start = Instant::now();
        let logger = logger(true);
        let registry = Arc::new(MockMetricsRegistry::new());
        let stopwatch = StopwatchMetrics::new(
            Logger::root(slog::Discard, slog::o!()),
            DEPLOYMENT_HASH.cheap_clone(),
            registry.clone(),
        );
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                log::info!(
                    "Start decode payload with size {} at {:?}",
                    data.payload.len(),
                    start.elapsed()
                );
                let eth_block: EthereumBlock = decode(&mut data.payload).unwrap();
                log::info!("Decoded payload at {:?}", start.elapsed());
                let arc_block = Arc::new(eth_block.block.clone());
                // let block_finality: Arc<<Chain as Blockchain>::Block> =
                //     Arc::new(BlockFinality::Final(arc_block.clone()));
                // let block_ptr = BlockPtr {
                //     hash: BlockHash(data.block_hash.as_bytes().into()),
                //     number: data.block_number as i32,
                // };
                // let data_sources = self.data_sources.clone();
                // log::info!("Cloned data_sources at {:?}", start.elapsed());
                // data_sources.into_iter().for_each(|data_source| {
                //     //wasm_instance for each datasource
                //     let mut wasm_instance: Option<WasmInstance<Chain>> = None;
                //     self.matching_block(
                //         &logger,
                //         &mut wasm_instance,
                //         &data_source,
                //         &eth_block,
                //         block_finality.clone(),
                //         &block_ptr,
                //         registry.cheap_clone(),
                //         stopwatch.cheap_clone(),
                //     );
                // });
            }
            _ => {}
        }

        log::info!(
            "{} Finished call handle_wasm_mapping in {:?}",
            &*COMPONENT_NAME,
            start.elapsed()
        );
        Ok(())
    }
}
impl EthereumWasmHandlerProxy {
    fn prepare_wasm_module(&mut self, data_source: &DataSource) -> Arc<ValidModule> {
        let cur_module = self.wasm_modules.get_mut(&data_source.name);
        match cur_module {
            None => {
                let valid_module =
                    Arc::new(ValidModule::new(&data_source.mapping.runtime).unwrap());
                log::info!(
                    "Import wasm module successfully: {:?}",
                    &valid_module.import_name_to_modules
                );
                self.wasm_modules
                    .insert(data_source.name.clone(), valid_module.clone());
                valid_module
            }
            Some(module) => module.clone(),
        }
    }
    fn get_ethereum_call(&mut self, data_source: &DataSource) -> HostFn {
        match self.ethereum_calls.get(&data_source.name) {
            None => {
                let ethereum_call = create_ethereum_call(data_source);
                self.ethereum_calls
                    .insert(data_source.name.clone(), ethereum_call.cheap_clone());
                ethereum_call
            }
            Some(host_fn) => host_fn.cheap_clone(),
        }
    }
    ///Create wasm_instance once by data_source
    fn prepare_wasm_instance(
        &mut self,
        wasm_instance: &mut Option<WasmInstance<Chain>>,
        data_source: &DataSource,
        registry: Arc<MockMetricsRegistry>,
        block_ptr: &BlockPtr,
    ) {
        if wasm_instance.is_none() {
            let valid_module = self.prepare_wasm_module(data_source);
            let ethereum_call = self.get_ethereum_call(data_source);
            *wasm_instance = Some(
                load_wasm(
                    &self.indexer_hash,
                    data_source,
                    self.templates.clone(),
                    self.store.clone(),
                    valid_module,
                    ethereum_call,
                    registry,
                    block_ptr,
                )
                .unwrap(),
            );
        }
    }
    fn matching_block(
        &mut self,
        logger: &Logger,
        wasm_instance: &mut Option<WasmInstance<Chain>>,
        data_source: &DataSource,
        eth_block: &EthereumBlock,
        block_finality: Arc<<Chain as Blockchain>::Block>,
        block_ptr: &BlockPtr,
        registry: Arc<MockMetricsRegistry>,
        stopwatch: StopwatchMetrics,
    ) {
        //Trigger block
        let block_trigger: <Chain as Blockchain>::TriggerData =
            EthereumTrigger::Block(block_ptr.cheap_clone(), EthereumBlockTriggerType::Every);
        match data_source.match_and_decode(&block_trigger, block_finality.clone(), &logger) {
            Ok(mapping_trigger) => {
                if let Some(trigger) = mapping_trigger {
                    log::info!("Block Mapping trigger found");
                    self.prepare_wasm_instance(
                        wasm_instance,
                        data_source,
                        registry.cheap_clone(),
                        block_ptr,
                    );
                    wasm_instance.as_mut().unwrap().handle_trigger(trigger);
                }
            }
            Err(err) => {
                log::error!("Try match EthereumTrigger::Block with error {:?}", err);
            }
        }

        //Mapping trigger log
        eth_block.logs.iter().for_each(|log| {
            let arc_log = Arc::new(log.clone());
            let trigger: <Chain as Blockchain>::TriggerData = EthereumTrigger::Log(arc_log);
            match data_source.match_and_decode(&trigger, block_finality.clone(), logger) {
                Ok(mapping_trigger) => {
                    if let Some(trigger) = mapping_trigger {
                        self.prepare_wasm_instance(
                            wasm_instance,
                            data_source,
                            registry.cheap_clone(),
                            block_ptr,
                        );
                        wasm_instance.as_mut().unwrap().handle_trigger(trigger);
                    }
                }
                Err(err) => {
                    log::error!("Try match EthereumTrigger::Log with error {:?}", err);
                }
            }
        });
        if let Some(instance) = wasm_instance {
            let mut context = instance.take_ctx();
            let _has_created_data_sources = context.ctx.state.has_created_data_sources();
            let data_source_infos = context.ctx.state.drain_created_data_sources();
            let ModificationsAndCache {
                modifications: mods,
                data_sources,
                entity_lfu_cache: _cache,
            } = context
                .ctx
                .state
                .entity_cache
                .as_modifications()
                .map_err(|e| {
                    log::error!("Error {:?}", e);
                    StoreError::Unknown(e.into())
                })
                .unwrap();

            // Transact entity modifications into the store
            if mods.len() > 0 {
                match self.store.transact_block_operations(
                    block_ptr.cheap_clone(),
                    mods,
                    stopwatch.cheap_clone(),
                    data_sources,
                    vec![],
                ) {
                    Ok(_) => log::info!("Transact block operation successfully"),
                    Err(err) => log::info!("Transact block operation with error {:?}", err),
                }
            }
            for ds_template_info in data_source_infos {
                let data_source = DataSource::try_from(ds_template_info).unwrap();
                let mut wasm_instance: Option<WasmInstance<Chain>> = None;
                self.matching_block(
                    &logger,
                    &mut wasm_instance,
                    &data_source,
                    &eth_block,
                    block_finality.clone(),
                    block_ptr,
                    registry.cheap_clone(),
                    stopwatch.cheap_clone(),
                );
                log::info!(
                    "New datasource #{} with source: {:?}",
                    self.data_sources.len() + 1,
                    &data_source.source
                );
                self.add_data_source(data_source);
            }
        }
    }
}
pub fn load_wasm(
    indexer_hash: &String,
    datasource: &DataSource,
    templates: Arc<Vec<DataSourceTemplate>>,
    store: Arc<dyn WritableStore>,
    valid_module: Arc<ValidModule>,
    ethereum_call: HostFn,
    registry: Arc<MockMetricsRegistry>,
    block_ptr: &BlockPtr,
    //link_resolver: Arc<dyn LinkResolverTrait>,
) -> Result<WasmInstance<Chain>, anyhow::Error> {
    let stopwatch_metrics = StopwatchMetrics::new(
        Logger::root(slog::Discard, slog::o!()),
        DeploymentHash::new("_indexer").unwrap(),
        registry.clone(),
    );
    let host_metrics = Arc::new(HostMetrics::new(
        registry.clone(),
        indexer_hash.as_str(),
        stopwatch_metrics,
    ));
    let network = match &datasource.network {
        None => String::from("ethereum"),
        Some(val) => val.clone(),
    };
    let host_exports = HostExports::new(
        indexer_hash.as_str(),
        datasource,
        network,
        Arc::clone(&templates),
        datasource.mapping.api_version.clone(),
    );
    //check if wasm module use import ethereum.call
    let host_fns: Vec<HostFn> = match valid_module.import_name_to_modules.get("ethereum.call") {
        None => Vec::new(),
        Some(_) => {
            //vec![create_ethereum_call(datasource)]
            vec![ethereum_call]
            //vec![create_mock_ethereum_call(datasource)]
        }
    };
    //datasource.mapping.requires_archive();
    let context = MappingContext {
        logger: Logger::root(slog::Discard, slog::o!()),
        block_ptr: block_ptr.cheap_clone(),
        host_exports: Arc::new(host_exports),
        state: BlockState::new(store, Default::default()),
        //proof_of_indexing: None,
        host_fns: Arc::new(host_fns),
    };
    let timeout = None;

    WasmInstance::from_valid_module_with_ctx(
        valid_module,
        context,
        host_metrics,
        timeout,
        //ExperimentalFeatures {
        //    allow_non_deterministic_ipfs: false,
        //},
    )
}

impl MessageHandler for EthereumHandlerProxy {
    fn handle_rust_mapping(
        &self,
        data: &mut GenericDataProto,
        store: &mut dyn Store,
    ) -> Result<(), Box<dyn Error>> {
        //println!("GenericDataProto{:?}", data);
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let block: EthereumBlock = decode(&mut data.payload).unwrap();
                log::info!(
                    "{} Received ETHEREUM BLOCK with block height: {:?}, hash: {:?}",
                    &*COMPONENT_NAME,
                    &block.block.number.unwrap(),
                    &block.block.hash.unwrap()
                );
                self.handler.handle_block(&block);
                for origin_transaction in block.block.transactions.clone() {
                    let transaction = EthereumTransaction {
                        version: block.version.clone(),
                        timestamp: block.timestamp,
                        // receipt: block.receipts.get(&origin_transaction.hash).cloned(),
                        receipt: Default::default(),
                        transaction: origin_transaction,
                    };
                    self.handler.handle_transaction(&transaction);
                }

                // Todo: add event for rust plugin, now do not support (use wasm plugin instead).
                // Create event
                // let logger = graph::log::logger(false);
                // let events = get_events(&block, data_source, &logger);
                // for event in events {
                //     log::debug!("Do event handler: Event address {:?}", &event.event.address);
                //     self.handler.handle_event(&event);
                // }
                store.flush(&data.block_hash, data.block_number)
            }
            _ => {
                log::warn!(
                    "{} Not support data type: {:?}",
                    &*COMPONENT_NAME,
                    &data.data_type
                );
                Err(Box::new(AdapterError::new(
                    format!("Not support data type: {:?}", &data.data_type).as_str(),
                )))
            }
        }
    }
}
