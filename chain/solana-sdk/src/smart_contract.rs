use std::error::Error;
use std::sync::Arc;
pub use transport::interface::{InstructionInterface, InstructionParser, InterfaceRegistrar};
pub use transport::TransportValue;

pub struct SmartContractProxy {
    pub parser: Box<dyn InstructionParser + Send + Sync>,
}
impl SmartContractProxy {
    pub fn new(parser: Box<dyn InstructionParser + Send + Sync>) -> SmartContractProxy {
        SmartContractProxy { parser }
    }
}
impl InstructionParser for SmartContractProxy {
    fn unpack_instruction(&self, content: &[u8]) -> Result<TransportValue, Box<dyn Error>> {
        println!(
            "start unpack_instruction, content len: {:?}",
            &content.len()
        );
        let result = self.parser.unpack_instruction(content);
        match &result {
            Ok(value) => {
                println!("value: {:?}", value);
            }
            Err(err) => {
                println!("err: {:?}", err);
            }
        }
        result
    }
}

#[derive(Clone)]
pub struct SmartContractRegistrar {
    pub parser_proxies: Option<Arc<SmartContractProxy>>,
}
impl SmartContractRegistrar {
    pub fn new() -> Self {
        Self {
            parser_proxies: None,
        }
    }
}

impl InterfaceRegistrar for SmartContractRegistrar {
    fn register_parser(&mut self, handler: Box<dyn InstructionParser + Send + Sync>) {
        println!("start register_parser");
        self.parser_proxies = Some(Arc::new(SmartContractProxy::new(handler)));
    }
}
