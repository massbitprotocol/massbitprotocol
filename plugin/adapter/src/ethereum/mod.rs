use crate::core::{AdapterError, MessageHandler};
pub use crate::stream_mod::{DataType, GenericDataProto};
use crate::EthereumWasmHandlerProxy;
use ethabi::{Address, LogParam, Token, Uint};
use graph::blockchain::{Blockchain, DataSource as DataSourceTrait, HostFn};
use graph::components::ethereum::LightEthereumBlockExt;
use graph::data::subgraph::{DeploymentHash, Mapping};
use graph_chain_ethereum::{
    chain::BlockFinality,
    trigger::{EthereumTrigger, MappingTrigger},
    Chain, DataSource, DataSourceTemplate,
};
//use graph_runtime_wasm::WasmInstance;
use graph::blockchain::types::{BlockHash, BlockPtr};
use graph::cheap_clone::CheapClone;
use graph::components::metrics::stopwatch::StopwatchMetrics;
use graph::components::metrics::MetricsRegistry;
use graph::components::store::{ModificationsAndCache, StoreError, WritableStore};
use graph::components::subgraph::{BlockState, DataSourceTemplateInfo, HostMetrics};
use graph::log::logger;
use graph_chain_ethereum::trigger::EthereumBlockTriggerType;
use graph_mock::MockMetricsRegistry;
use graph_runtime_wasm::ValidModule;
use libloading::Library;
use massbit_chain_ethereum::data_type::{
    decode, get_events, EthereumBlock, EthereumEvent, EthereumTransaction,
};
use massbit_common::prelude::anyhow;
use massbit_runtime_wasm::host_exports::create_ethereum_call;
use massbit_runtime_wasm::prelude::{Logger, Version};
use massbit_runtime_wasm::store::postgres::store_builder::*;
use massbit_runtime_wasm::{slog, HostExports, MappingContext, WasmInstance};
use std::convert::TryFrom;
use std::str::FromStr;
use std::time::Instant;
use std::{error::Error, sync::Arc};

const API_VERSION_0_0_4: Version = Version::new(0, 0, 4);
const API_VERSION_0_0_5: Version = Version::new(0, 0, 5);

crate::prepare_adapter!(Ethereum, {
    handle_block: EthereumBlock,
    handle_transaction: EthereumTransaction,
    handle_event: EthereumEvent
});
impl MessageHandler for EthereumWasmHandlerProxy {
    fn handle_wasm_mapping(&mut self, data: &mut GenericDataProto) -> Result<(), Box<dyn Error>> {
        log::info!("{} call handle_wasm_mapping", &*COMPONENT_NAME);
        let logger = logger(true);
        let registry = Arc::new(MockMetricsRegistry::new());
        let stopwatch = StopwatchMetrics::new(
            Logger::root(slog::Discard, slog::o!()),
            DEPLOYMENT_HASH.cheap_clone(),
            registry.clone(),
        );
        let start = Instant::now();
        let data_sources = self.data_sources.clone();
        data_sources.iter().for_each(|data_source| {
            let valid_module = self.prepare_wasm_module(data_source);
            let ethereum_call = self.get_ethereum_call(data_source);
            let mut wasm_instance = load_wasm(
                &self.indexer_hash,
                data_source,
                self.templates.clone(),
                self.store.clone(),
                valid_module,
                ethereum_call,
                registry.cheap_clone(),
            )
            .unwrap();

            match DataType::from_i32(data.data_type) {
                Some(DataType::Block) => {
                    let eth_block: EthereumBlock = decode(&mut data.payload).unwrap();
                    let arc_block = Arc::new(eth_block.block.clone());
                    let block_finality: Arc<<Chain as Blockchain>::Block> =
                        Arc::new(BlockFinality::Final(arc_block.clone()));
                    let block_ptr_to = BlockPtr {
                        hash: BlockHash(data.block_hash.as_bytes().into()),
                        number: data.block_number as i32,
                    };

                    self.matching_block(
                        &logger,
                        &mut wasm_instance,
                        data_source,
                        &eth_block,
                        block_finality.clone(),
                        block_ptr_to,
                        registry.cheap_clone(),
                        stopwatch.cheap_clone(),
                    );
                    //Handle result in BlockState
                    /*
                    let mut context = wasm_instance.take_ctx();
                    //let mut state = context.ctx.state;
                    let has_created_data_sources = context.ctx.state.has_created_data_sources();
                    let data_source_infos = context.ctx.state.drain_created_data_sources();
                    for ds_template_info in data_source_infos {
                        let data_source = DataSource::try_from(ds_template_info).unwrap();
                        let valid_module = self.prepare_wasm_module(&data_source);
                        let ethereum_call = self.get_ethereum_call(&data_source);
                        let mut wasm_instance = load_wasm(
                            &self.indexer_hash,
                            &data_source,
                            self.templates.clone(),
                            self.store.clone(),
                            valid_module,
                            ethereum_call,
                            registry.cheap_clone(),
                        )
                        .unwrap();
                        self.matching_block(
                            &logger,
                            &mut wasm_instance,
                            &data_source,
                            &eth_block,
                            block_finality.clone(),
                            &block_ptr_to,
                        );
                        println!("New datasource with source: {:?}", &data_source.source);
                        self.add_data_source(data_source);
                        println!("Total datasource: {:?}", self.data_sources.len());
                    }
                    let state = context.ctx.state;
                    let ModificationsAndCache {
                        modifications: mods,
                        data_sources,
                        entity_lfu_cache: cache,
                    } = state
                        .entity_cache
                        .as_modifications()
                        .map_err(|e| {
                            log::error!("Error {:?}", e);
                            StoreError::Unknown(e.into())
                        })
                        .unwrap();
                    let stopwatch = StopwatchMetrics::new(
                        Logger::root(slog::Discard, slog::o!()),
                        DEPLOYMENT_HASH.cheap_clone(),
                        registry.clone(),
                    );
                    // Transact entity modifications into the store
                    if mods.len() > 0 {
                        self.store.transact_block_operations(
                            block_ptr_to,
                            mods,
                            stopwatch.cheap_clone(),
                            data_sources,
                            vec![],
                        );
                    }
                     */
                }
                _ => {}
            }
        });
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
    fn matching_block(
        &mut self,
        logger: &Logger,
        wasm_instance: &mut WasmInstance<Chain>,
        data_source: &DataSource,
        eth_block: &EthereumBlock,
        block_finality: Arc<<Chain as Blockchain>::Block>,
        block_ptr_to: BlockPtr,
        registry: Arc<MockMetricsRegistry>,
        stopwatch: StopwatchMetrics,
    ) {
        //Trigger block
        let block_trigger: <Chain as Blockchain>::TriggerData =
            EthereumTrigger::Block(block_ptr_to.clone(), EthereumBlockTriggerType::Every);
        let mapping_trigger = data_source
            .match_and_decode(&block_trigger, block_finality.clone(), &logger)
            .unwrap();
        if let Some(trigger) = mapping_trigger {
            log::info!("Block Mapping trigger found");
            wasm_instance.handle_trigger(trigger);
            /*
            match wasm_instance {
                Some(ref mut instance) => {
                    instance.handle_trigger(trigger);
                }
                None => {
                    let mut instance = load_wasm(
                        &self.indexer_hash,
                        data_source,
                        self.templates.clone(),
                        self.store.clone(),
                        registry.cheap_clone(),
                    )
                    .unwrap();
                    &instance.handle_trigger(trigger);
                    wasm_instance = Some(instance);
                }
            }
             */
        }
        //Mapping trigger log
        eth_block.logs.iter().for_each(|log| {
            let arc_log = Arc::new(log.clone());
            let trigger: <Chain as Blockchain>::TriggerData = EthereumTrigger::Log(arc_log);
            let mapping_trigger = data_source
                .match_and_decode(&trigger, block_finality.clone(), logger)
                .unwrap();
            if let Some(trigger) = mapping_trigger {
                log::info!("Log Mapping trigger found");
                wasm_instance.handle_trigger(trigger);
                /*
                match wasm_instance {
                    Some(ref mut instance) => {
                        instance.handle_trigger(trigger);
                    }
                    None => {
                        let mut instance = load_wasm(
                            &self.indexer_hash,
                            data_source,
                            self.templates.clone(),
                            self.store.clone(),
                            registry.cheap_clone(),
                        )
                        .unwrap();
                        &instance.handle_trigger(trigger);
                        wasm_instance = Some(instance);
                    }
                }

                 */
            }
        });
        let mut context = wasm_instance.take_ctx();
        let has_created_data_sources = context.ctx.state.has_created_data_sources();
        let data_source_infos = context.ctx.state.drain_created_data_sources();
        let ModificationsAndCache {
            modifications: mods,
            data_sources,
            entity_lfu_cache: cache,
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
                block_ptr_to.cheap_clone(),
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
            let valid_module = self.prepare_wasm_module(&data_source);
            let ethereum_call = self.get_ethereum_call(&data_source);
            let mut wasm_instance = load_wasm(
                &self.indexer_hash,
                &data_source,
                self.templates.clone(),
                self.store.clone(),
                valid_module,
                ethereum_call,
                registry.cheap_clone(),
            )
            .unwrap();
            self.matching_block(
                &logger,
                &mut wasm_instance,
                &data_source,
                &eth_block,
                block_finality.clone(),
                block_ptr_to.cheap_clone(),
                registry.cheap_clone(),
                stopwatch.cheap_clone(),
            );
            println!("New datasource with source: {:?}", &data_source.source);
            self.add_data_source(data_source);
            println!("Total datasource: {:?}", self.data_sources.len());
        }
    }
}
/*
impl EthereumWasmHandlerProxy {
    fn prepare_wasm_instance(
        &mut self,
        data_source: &DataSource,
        registry: Arc<MockMetricsRegistry>,
    ) -> &mut WasmInstance<Chain> {
        let cur_instance = self.wasm_instances.get_mut(&data_source.name);
        match cur_instance {
            None => {
                let mut instance = load_wasm(
                    &self.indexer_hash,
                    data_source,
                    self.templates.clone(),
                    self.store.clone(),
                    registry.cheap_clone(),
                )
                .unwrap();
                self.wasm_instances
                    .insert(data_source.name.clone(), instance);
            }
            _ => {}
        };
        self.wasm_instances.get_mut(&data_source.name).unwrap()
    }
}
*/
/*
pub fn extract_wasm_instance(wasm_instance: &mut Option<WasmInstance<Chain>>) -> &mut WasmInstance<Chain> {
    match wasm_instance {
        Some(mut instance) => instance,
        None => {
            let mut instance = load_wasm(
                &self.indexer_hash,
                data_source,
                self.templates.clone(),
                self.store.clone(),
                registry.cheap_clone(),
            )
                .unwrap();
            wasm_instance = Some(instance);
            &instance.handle_trigger(trigger);
        }
    }
}
 */
pub fn load_wasm(
    indexer_hash: &String,
    datasource: &DataSource,
    templates: Arc<Vec<DataSourceTemplate>>,
    store: Arc<dyn WritableStore>,
    valid_module: Arc<ValidModule>,
    ethereum_call: HostFn,
    registry: Arc<MockMetricsRegistry>,
    //link_resolver: Arc<dyn LinkResolverTrait>,
) -> Result<WasmInstance<Chain>, anyhow::Error> {
    let api_version = API_VERSION_0_0_4.clone();

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
        api_version,
    );
    /*
    let valid_module = Arc::new(ValidModule::new(&datasource.mapping.runtime).unwrap());
    log::info!(
        "Import wasm module successfully: {:?}",
        &valid_module.import_name_to_modules
    );
     */
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
        block_ptr: BlockPtr {
            hash: Default::default(),
            number: datasource.source.start_block,
        },
        host_exports: Arc::new(host_exports),
        //state: IndexerState::new(store, Default::default()),
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

/*
impl MessageHandler for EthereumWasmHandlerProxy {
    fn handle_wasm_mapping(
        &self,
        wasm_instance: &mut WasmInstance<Chain>,
        data_source: &DataSource,
        data: &mut GenericDataProto,
    ) -> Result<(), Box<dyn Error>> {
        log::info!("{} call handle_wasm_mapping", &*COMPONENT_NAME);
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let eth_block: EthereumBlock = decode(&mut data.payload).unwrap();
                let arc_block = Arc::new(eth_block.block);
                let block_finality: Arc<<Chain as Blockchain>::Block> =
                    Arc::new(BlockFinality::Final(arc_block.clone()));
                let block_ptr_to = BlockPtr {
                    hash: BlockHash(data.block_hash.as_bytes().into()),
                    number: data.block_number as i32,
                };
                let logger = logger(true);
                /*
                let params = vec![
                    LogParam {
                        name: "token0".to_string(),
                        value: Token::Address(
                            Address::from_str("e0b7927c4af23765cb51314a0e0521a9645f0e2b").unwrap(),
                        ),
                    },
                    LogParam {
                        name: "token1".to_string(),
                        value: Token::Address(
                            Address::from_str("7fc66500c84a76ad7e9c93437bfc5ac33e2ddae0").unwrap(),
                        ),
                    },
                    LogParam {
                        name: "pair".to_string(),
                        value: Token::Address(
                            Address::from_str("7fc66500c84a76ad7e9c93437bfc5ac33e2ddbe0").unwrap(),
                        ),
                    },
                    LogParam {
                        name: "param3".to_string(),
                        value: Token::Int(Uint::from(123)),
                    },
                ];
                 */
                //Trigger block
                let block_trigger: <Chain as Blockchain>::TriggerData =
                    EthereumTrigger::Block(block_ptr_to, EthereumBlockTriggerType::Every);
                let mapping_trigger = data_source
                    .match_and_decode(&block_trigger, block_finality.clone(), &logger)
                    .unwrap();
                if let Some(trigger) = mapping_trigger {
                    log::info!("Block Mapping trigger found");
                    wasm_instance.handle_trigger(trigger);
                }
                //Mapping trigger log
                eth_block.logs.iter().for_each(|log| {
                    //if let Some(transaction) = arc_block.transaction_for_log(log) {
                    //let arc_tran = Arc::new(transaction.clone());
                    let arc_log = Arc::new(log.clone());
                    let trigger: <Chain as Blockchain>::TriggerData = EthereumTrigger::Log(arc_log);
                    let mapping_trigger = data_source
                        .match_and_decode(&trigger, block_finality.clone(), &logger)
                        .unwrap();
                    if let Some(trigger) = mapping_trigger {
                        log::info!("Log Mapping trigger found");
                        wasm_instance.handle_trigger(trigger);
                    }
                    //};
                });
                //Trigger Call
            }
            Some(DataType::Event) => {
                log::info!("Found event");
                /*
                data_source
                    .mapping()
                    .event_handlers
                    .iter()
                    .for_each(|handler| {});
                 */
            }
            Some(DataType::Transaction) => {
                log::info!("Found transaction");
            }
            _ => {}
        }
        let ctx = wasm_instance.take_ctx();
        //let state = wasm_instance.take_ctx().ctx.state;
        // If new data sources have been created, restart the subgraph after this block.
        // This is necessary to re-create the block stream.
        let host_metrics = ctx.host_metrics.clone();
        let mut state = ctx.ctx.state;
        let has_created_data_sources = state.has_created_data_sources();
        for ds_template_info in state.drain_created_data_sources() {
            let data_source = DataSource::try_from(ds_template_info)?;
            println!("{:?}", &data_source);
        }

        /*
        let ModificationsAndCache {
            modifications: mods,
            data_sources,
            entity_lfu_cache: cache,
        } = state.entity_cache.as_modifications().map_err(|e| {
            log::error!("Error {:?}", e);
            StoreError::Unknown(e.into())
        })?;

        // Transact entity modifications into the store
        if mods.len() > 0 {
            store.transact_block_operations(
                block_ptr_to,
                mods,
                stopwatch.cheap_clone(),
                data_sources,
                vec![],
            );
        }
         */
        Ok(())
    }
}
 */
/*
fn create_dynamic_data_sources<C: Blockchain>(
    //logger: Logger,
    //ctx: &mut IndexingContext<T, C>,
    //host_metrics: Arc<HostMetrics>,
    created_data_sources: Vec<DataSourceTemplateInfo<C>>,
) -> Result<Vec<C::DataSource>, Error> {
    let mut data_sources = vec![];
    let mut runtime_hosts = vec![];

    for info in created_data_sources {
        // Try to instantiate a data source from the template
        let data_source = C::DataSource::try_from(info)?;
        println!("Created datasource {:?}", &data_source);
        /*
        // Try to create a runtime host for the data source
        let host = ctx.state.instance.add_dynamic_data_source(
            &logger,
            data_source.clone(),
            ctx.inputs.templates.clone(),
            host_metrics.clone(),
        )?;

        match host {
            Some(host) => {
                data_sources.push(data_source);
                runtime_hosts.push(host);
            }
            None => {
                fail_point!("error_on_duplicate_ds", |_| Err(anyhow!("duplicate ds")));
                warn!(
                    logger,
                    "no runtime hosted created, there is already a runtime host instantiated for \
                     this data source";
                    "name" => &data_source.name(),
                    "address" => &data_source.address()
                        .map(|address| hex::encode(address))
                        .unwrap_or("none".to_string()),
                )
            }
        }
         */
    }

    Ok((data_sources, runtime_hosts))
}
 */
impl MessageHandler for EthereumHandlerProxy {
    fn handle_rust_mapping(&self, data: &mut GenericDataProto) -> Result<(), Box<dyn Error>> {
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
                        receipt: block.receipts.get(&origin_transaction.hash).cloned(),
                        transaction: origin_transaction,
                    };
                    self.handler.handle_transaction(&transaction);
                }

                // Create event
                // let events = get_events(&block);
                // for event in events {
                //     log::debug!("Do event handler: Event address {:?}", &event.event.address);
                //     self.handler.handle_event(&event);
                // }

                Ok(())
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
