use crate::core::{AdapterError, MessageHandler};
pub use crate::stream_mod::{DataType, GenericDataProto};
use libloading::Library;
use massbit_chain_ethereum::data_type::{decode, EthereumBlock};
use std::{error::Error, sync::Arc};

crate::prepare_adapter!(Ethereum, { handle_block: EthereumBlock });

impl MessageHandler for EthereumHandlerProxy {
    fn handle_message(&self, data: &mut GenericDataProto) -> Result<(), Box<dyn Error>> {
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
