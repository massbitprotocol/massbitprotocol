use crate::core::{AdapterError, MessageHandler};
pub use crate::stream_mod::{DataType, GenericDataProto};
//use crate::EthereumWasmHandlerProxy;
use libloading::Library;
use massbit_chain_ethereum::data_type::{decode, EthereumBlock, EthereumTransaction};
//use massbit_runtime_wasm::chain::ethereum::{trigger::MappingTrigger, Chain};
//use massbit_runtime_wasm::indexer::manifest::{Mapping, MappingBlockHandler};
//use massbit_runtime_wasm::module::WasmInstance;

use std::{error::Error, sync::Arc};

crate::prepare_adapter!(Ethereum, { handle_block: EthereumBlock, handle_transaction: EthereumTransaction});
/*
impl MessageHandler for EthereumWasmHandlerProxy {
    fn handle_wasm_mapping(
        &self,
        wasm_instance: &mut WasmInstance<Chain>,
        mapping: &Mapping,
        data: &mut GenericDataProto,
    ) -> Result<(), Box<dyn Error>> {
        log::info!("{} call handle_wasm_mapping", &*COMPONENT_NAME);

        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                mapping.event_handlers.iter().for_each(|handler| {
                    let block_ext: EthereumBlock = decode(&mut data.payload).unwrap();
                    log::info!("Block {:?}", &block_ext.block);
                    let block_handler = MappingBlockHandler {
                        handler: handler.handler.clone(),
                        filter: None,
                    };
                    let trigger = MappingTrigger::Block {
                        block: Arc::new(block_ext.block),
                        handler: block_handler,
                    };
                    wasm_instance.handle_trigger(trigger);
                });
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
                mapping.event_handlers.iter().for_each(|handler| {});
            }
            Some(DataType::Transaction) => {}
            _ => {}
        }
        Ok(())
    }
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
                for origin_transaction in block.block.transactions {
                    let transaction = EthereumTransaction {
                        version: block.version.clone(),
                        timestamp: block.timestamp,
                        receipt: block.receipts.get(&origin_transaction.hash).cloned(),
                        transaction: origin_transaction,
                    };
                    self.handler.handle_transaction(&transaction);
                }

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
