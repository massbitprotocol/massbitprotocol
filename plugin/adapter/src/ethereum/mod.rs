use crate::core::{AdapterError, MessageHandler};
pub use crate::stream_mod::{DataType, GenericDataProto};
use crate::EthereumWasmHandlerProxy;
use ethabi::{Address, LogParam, Token, Uint};
use graph::blockchain::{Blockchain, DataSource as DataSourceTrait};
use graph::components::ethereum::LightEthereumBlockExt;
use graph::data::subgraph::Mapping;
use graph_chain_ethereum::{
    chain::BlockFinality,
    trigger::{EthereumTrigger, MappingTrigger},
    Chain, DataSource,
};
//use graph_runtime_wasm::WasmInstance;
use graph::blockchain::types::{BlockHash, BlockPtr};
use graph::log::logger;
use graph_chain_ethereum::trigger::EthereumBlockTriggerType;
use libloading::Library;
use massbit_chain_ethereum::data_type::{
    decode, get_events, EthereumBlock, EthereumEvent, EthereumTransaction,
};
use massbit_runtime_wasm::WasmInstance;
use std::str::FromStr;
use std::{error::Error, sync::Arc};
crate::prepare_adapter!(Ethereum, {
    handle_block: EthereumBlock,
    handle_transaction: EthereumTransaction,
    handle_event: EthereumEvent
});

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
        Ok(())
    }
}

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
