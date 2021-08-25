use crate::core::{AdapterError, MessageHandler};
pub use crate::stream_mod::{DataType, GenericDataProto};
use crate::EthereumWasmHandlerProxy;
use ethabi::{Address, LogParam, Token, Uint};
use graph::blockchain::DataSource as DataSourceTrait;
use graph::components::ethereum::LightEthereumBlockExt;
use graph::data::subgraph::Mapping;
use graph_chain_ethereum::{
    trigger::MappingTrigger, trigger::MappingTrigger::Log, Chain, DataSource,
};
//use graph_runtime_wasm::WasmInstance;
use libloading::Library;
use massbit_chain_ethereum::data_type::{
    decode, get_events, EthereumBlock, EthereumEvent, EthereumTransaction,
};
use massbit_runtime_wasm::WasmInstance;
//use massbit_runtime_wasm::module::WasmInstance;
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
        datasource: &DataSource,
        data: &mut GenericDataProto,
    ) -> Result<(), Box<dyn Error>> {
        log::info!("{} call handle_wasm_mapping", &*COMPONENT_NAME);
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let eth_block: EthereumBlock = decode(&mut data.payload).unwrap();
                let arc_block = Arc::new(eth_block.block);
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
                eth_block.logs.iter().for_each(|log| {
                    if let Some(transaction) = arc_block.transaction_for_log(log) {
                        let arc_log = Arc::new(log.clone());
                        let arc_tran = Arc::new(transaction.clone());
                        datasource
                            .mapping()
                            .event_handlers
                            .iter()
                            .for_each(|handler| {
                                let trigger = MappingTrigger::Log {
                                    block: Arc::clone(&arc_block),
                                    transaction: Arc::clone(&arc_tran),
                                    log: Arc::clone(&arc_log),
                                    params: params.clone(),
                                    handler: handler.clone(),
                                };
                                wasm_instance.handle_trigger(trigger);
                            });
                    }
                });

                /*
                let events = get_events(&eth_block);
                for event in events {

                }
                 */
                /*
                for handler in mapping.block_handlers.iter() {
                    let block_ext: EthereumBlock = decode(&mut data.payload).unwrap();
                    let trigger = MappingTrigger::Block {
                        block: Arc::new(block_ext.block),
                        handler: handler.clone(),
                    };
                    &wasm_instance.handle_trigger(trigger);
                }
                 */
            }
            Some(DataType::Event) => {
                datasource
                    .mapping()
                    .event_handlers
                    .iter()
                    .for_each(|handler| {});
            }
            Some(DataType::Transaction) => {}
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
