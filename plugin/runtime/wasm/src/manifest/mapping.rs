use crate::graph::cheap_clone::CheapClone;
use crate::graph::util::ethereum::string_to_h256;
use crate::prelude::web3::ethabi::ethereum_types::H256;
use crate::prelude::{Arc, Version};
use anyhow::{anyhow, Error};
use ethabi::{Address, Contract};
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BlockHandlerFilter {
    // Call filter will trigger on all blocks where the data source contract
    // address has been called
    Call,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MappingABI {
    pub name: String,
    pub contract: Contract,
}
#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingBlockHandler {
    pub handler: String,
    pub filter: Option<BlockHandlerFilter>,
}
impl MappingBlockHandler {
    pub fn from_value(value: &serde_yaml::Value) -> MappingBlockHandler {
        MappingBlockHandler {
            handler: value["handler"].as_str().unwrap().to_string(),
            filter: None,
        }
    }
}
#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingCallHandler {
    pub function: String,
    pub handler: String,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct MappingEventHandler {
    pub event: String,
    pub topic0: Option<H256>,
    pub handler: String,
}
impl MappingEventHandler {
    pub fn from_value(value: &serde_yaml::Value) -> MappingEventHandler {
        MappingEventHandler {
            event: value["event"].as_str().unwrap().to_string(),
            topic0: None,
            handler: value["handler"].as_str().unwrap().to_string(),
        }
    }
}
impl MappingEventHandler {
    pub fn topic0(&self) -> H256 {
        self.topic0
            .unwrap_or_else(|| string_to_h256(&self.event.replace("indexed ", "")))
    }
}

#[derive(Clone, Debug)]
pub struct Mapping {
    pub kind: String,
    pub api_version: Version,
    pub language: String,
    pub entities: Vec<String>,
    pub abis: Vec<Arc<MappingABI>>,
    pub block_handlers: Vec<MappingBlockHandler>,
    pub call_handlers: Vec<MappingCallHandler>,
    pub event_handlers: Vec<MappingEventHandler>,
    pub runtime: Arc<Vec<u8>>,
}
impl Mapping {
    pub fn from_value(value: &serde_yaml::Value) -> Mapping {
        let block_handlers = match value["blockHandlers"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|val| MappingBlockHandler::from_value(val))
                .collect::<Vec<MappingBlockHandler>>(),
            _ => Vec::default(),
        };
        let event_handlers = match value["eventHandlers"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|val| MappingEventHandler::from_value(val))
                .collect::<Vec<MappingEventHandler>>(),
            _ => Vec::default(),
        };
        Mapping {
            kind: "".to_string(),
            api_version: Version::new(0, 0, 4),
            language: value["language"].as_str().unwrap().to_string(),
            entities: vec![],
            abis: vec![],
            block_handlers,
            call_handlers: vec![],
            event_handlers,
            runtime: Arc::new(vec![]),
        }
    }
}
impl Mapping {
    pub fn requires_archive(&self) -> bool {
        self.calls_host_fn("ethereum.call")
    }

    fn calls_host_fn(&self, host_fn: &str) -> bool {
        use wasmparser::Payload;

        let runtime = self.runtime.as_ref().as_ref();

        for payload in wasmparser::Parser::new(0).parse_all(runtime) {
            match payload.unwrap() {
                Payload::ImportSection(s) => {
                    for import in s {
                        let import = import.unwrap();
                        if import.field == Some(host_fn) {
                            return true;
                        }
                    }
                }
                _ => (),
            }
        }

        false
    }

    pub fn has_call_handler(&self) -> bool {
        !self.call_handlers.is_empty()
    }

    pub fn has_block_handler_with_call_filter(&self) -> bool {
        self.block_handlers
            .iter()
            .any(|handler| matches!(handler.filter, Some(BlockHandlerFilter::Call)))
    }

    pub fn find_abi(&self, abi_name: &str) -> Result<Arc<MappingABI>, Error> {
        Ok(self
            .abis
            .iter()
            .find(|abi| abi.name == abi_name)
            .ok_or_else(|| anyhow!("No ABI entry with name `{}` found", abi_name))?
            .cheap_clone())
    }
}
