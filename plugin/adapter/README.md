## Chain indexer adapter
Each adapter  has 2 components and created by macro prepare_adapter.
```
{$Adapter}Handler: Trait contains all processing method.
{$Adapter}HandlerProxy: Implement all method in trait {$AdapterHandler} 
and forward method call to inner handler which is injected into proxy 
by custom code.
```
Macro create_adapters create an exported trait PluginRegistrar with method 
for registry chain adapter in the form
```
register_$adapter_handler
```
## Usage

1. Create sperate directory for adapter with single file mod.rs for example
```
use libloading::Library;
use paste::paste;
use std::{error::Error, sync::Arc};
use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction, SolanaEncodedBlock, decode, convert_solana_encoded_block_to_solana_block};
pub use crate::stream_mod::{DataType, BlockResponse};
use std::result::Result::Err;
use crate::core::MessageHandler;

crate::prepare_adapter!(Solana, {
     handle_block:SolanaBlock,
     handle_transaction:SolanaTransaction,
     handle_log_messages:SolanaLogMessages
});

impl MessageHandler for SolanaHandlerProxy {
     fn handle_message(&self, data: &mut BlockResponse) -> Result<(), Box<dyn Error>> {
          match DataType::from_i32(data.data_type) {
               Some(DataType::Block) => {
                    let encoded_block: SolanaEncodedBlock = decode(&mut data.payload).unwrap();
                    let block = convert_solana_encoded_block_to_solana_block(encoded_block); // Decoding
                    log::info!("{} Received SOLANA BLOCK with block height: {:?}, hash: {:?}",&*COMPONENT_NAME, &block.block.block_height.unwrap(), &block.block.blockhash);
                    self.handler.handle_block(&block);
                    let mut print_flag = true;
                    for origin_transaction in block.clone().block.transactions {
                         let origin_log_messages = origin_transaction.meta.clone().unwrap().log_messages;
                         let transaction = SolanaTransaction {
                              block_number: ((&block).block.block_height.unwrap() as u32),
                              transaction: origin_transaction.clone(),
                              log_messages: origin_log_messages.clone(),
                              success: false
                         };
                         let log_messages = SolanaLogMessages {
                              block_number: ((&block).block.block_height.unwrap() as u32),
                              log_messages: origin_log_messages.clone(),
                              transaction: origin_transaction.clone(),
                         };
                         if print_flag {
                              log::info!("{} Recieved SOLANA TRANSACTION with Block number: {:?}, transaction: {:?}", &*COMPONENT_NAME, &transaction.block_number, &transaction.transaction.transaction.signatures);
                              log::info!("{} Recieved SOLANA LOG_MESSAGES with Block number: {:?}, log_messages: {:?}", &*COMPONENT_NAME, &log_messages.block_number, &log_messages.log_messages.clone().unwrap().get(0));
                              print_flag = false;
                         }
                         self.handler.handle_transaction(&transaction);
                         self.handler.handle_log_messages(&log_messages);
                    }
                    Ok(())
               },
               _ => {
                    log::warn!("{} Not support data type: {:?}", &*COMPONENT_NAME, &data.data_type);
                    Err(Box::new(crate::AdapterError::new(format!("Not support data type: {:?}", &data.data_type).as_str())))
               }
          }
     }
}
```
2. Add new adapter name to file src/lib.rs
```
................
//Add new chain name in CamelCase here
crate::create_adapters!(Matic, Bsc, Ethereum, Solana);
................
