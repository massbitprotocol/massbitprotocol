use libloading::Library;
use log::log;
use std::error::Error;
use std::sync::Arc;
pub use transport::interface::{InstructionInterface, InstructionParser, InterfaceRegistrar};
pub use transport::TransportValue;

pub struct SmartContractProxy {
    pub parser: Box<dyn InstructionParser>,
}
impl SmartContractProxy {
    pub fn new(parser: Box<dyn InstructionParser>) -> SmartContractProxy {
        SmartContractProxy { parser }
    }
}
impl InstructionParser for SmartContractProxy {
    fn unpack_instruction(&self, content: &[u8]) -> Result<TransportValue, anyhow::Error> {
        log::info!(
            "start unpack_instruction, content len: {:?}",
            &content.len()
        );
        let result = self.parser.unpack_instruction(content);
        match &result {
            Ok(value) => {
                log::info!("value: {:?}", value);
            }
            Err(err) => {
                log::error!("err: {:?}", err);
            }
        }
        result
    }
}

#[derive(Clone)]
pub struct SmartContractRegistrar {
    pub parser_proxies: Option<Arc<SmartContractProxy>>,
    _lib: Arc<Library>,
}
impl SmartContractRegistrar {
    pub fn new(lib: Arc<Library>) -> Self {
        Self {
            parser_proxies: None,
            _lib: lib,
        }
    }
}

impl InterfaceRegistrar for SmartContractRegistrar {
    fn register_parser(&mut self, handler: Box<dyn InstructionParser>) {
        self.parser_proxies = Some(Arc::new(SmartContractProxy::new(handler)));
    }
}
