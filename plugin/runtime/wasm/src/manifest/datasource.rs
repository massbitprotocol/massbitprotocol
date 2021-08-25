//use super::{Mapping, MappingABI};
//use crate::graph::data::store::Entity;
//use crate::graph::prelude::BlockNumber;
//use crate::indexer::blockchain::{self, Blockchain};
//use crate::indexer::manifest::{DataSourceTemplateInfo, StoredDynamicDataSource};
//use crate::prelude::anyhow::Context;
//use crate::prelude::serde_yaml::Value;
//use crate::prelude::Error;
use graph::components::store::BlockNumber;
use graph::components::subgraph::Entity;
use graph::data::subgraph::{
    Mapping, MappingABI, MappingBlockHandler, MappingCallHandler, MappingEventHandler, Source,
    TemplateSource,
};
use graph::prelude::{ethabi::Contract, DataSourceTemplateInfo};
use graph_chain_ethereum::Chain;
use graph_chain_ethereum::{DataSource, DataSourceTemplate};
use massbit_common::prelude::anyhow::Context;
use massbit_common::prelude::ethabi::Address;
use massbit_common::prelude::serde_derive::{Deserialize, Serialize};
use massbit_common::prelude::serde_yaml;
use massbit_common::prelude::serde_yaml::Value;
use serde::de;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::default::Default;
use std::str::FromStr;
use std::sync::Arc;
//use crate::indexer::manifest::{MappingBlockHandler, MappingEventHandler};
use crate::prelude::Version;

//use crate::indexer::manifest::DataSourceTemplateInfo;

//use ethabi::Address;
pub type DataSourceContext = Entity;

pub trait FromValue<T, S> {
    type Error;
    fn try_from(value: &T) -> Result<S, Self::Error>;
}

/*
#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub struct Source {
    /// The contract address for the data source. We allow data sources
    /// without an address for 'wildcard' triggers that catch all possible
    /// events with the given `abi`
    #[serde(default, deserialize_with = "deserialize_address")]
    pub address: Option<Address>,
    pub abi: String,
    #[serde(rename = "startBlock", default)]
    pub start_block: BlockNumber,
}
*/
impl FromValue<serde_yaml::Value, Source> for Source {
    type Error = anyhow::Error;
    fn try_from(value: &serde_yaml::Value) -> Result<Source, Self::Error> {
        let address = match &value["address"] {
            Value::String(addr) => Some(
                Address::from_str(addr.trim_start_matches("0x")).with_context(|| {
                    format!(
                        "Failed to create address from value `{}`, invalid address provided",
                        addr
                    )
                })?,
            ),
            _ => None,
        };
        Ok(Source {
            address,
            abi: value["abi"].as_str().unwrap().to_string(),
            start_block: value["startBlock"].as_i64().unwrap() as i32,
        })
    }
}

impl FromValue<serde_yaml::Value, MappingBlockHandler> for MappingBlockHandler {
    type Error = anyhow::Error;

    fn try_from(value: &serde_yaml::Value) -> Result<MappingBlockHandler, Self::Error> {
        Ok(MappingBlockHandler {
            handler: value["handler"].as_str().unwrap().to_string(),
            filter: None,
        })
    }
}
impl FromValue<serde_yaml::Value, MappingEventHandler> for MappingEventHandler {
    type Error = anyhow::Error;

    fn try_from(value: &serde_yaml::Value) -> Result<MappingEventHandler, Self::Error> {
        Ok(MappingEventHandler {
            event: value["event"].as_str().unwrap().to_string(),
            topic0: None,
            handler: value["handler"].as_str().unwrap().to_string(),
        })
    }
}
impl FromValue<serde_yaml::Value, Mapping> for Mapping {
    type Error = anyhow::Error;
    fn try_from(value: &serde_yaml::Value) -> Result<Mapping, Self::Error> {
        let block_handlers = match value["blockHandlers"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|val| MappingBlockHandler::try_from(val).unwrap())
                .collect::<Vec<MappingBlockHandler>>(),
            _ => Vec::default(),
        };
        let event_handlers = match value["eventHandlers"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|val| MappingEventHandler::try_from(val).unwrap())
                .collect::<Vec<MappingEventHandler>>(),
            _ => Vec::default(),
        };
        Ok(Mapping {
            kind: "".to_string(),
            api_version: Version::new(0, 0, 4),
            language: value["language"].as_str().unwrap().to_string(),
            entities: vec![],
            abis: vec![],
            block_handlers,
            call_handlers: vec![],
            event_handlers,
            runtime: Arc::new(vec![]),
            link: Default::default(),
        })
    }
}
/*
#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize)]
pub struct TemplateSource {
    pub abi: String,
}
#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize)]
pub struct BaseDataSourceTemplate<M> {
    pub kind: String,
    pub network: Option<String>,
    pub name: String,
    pub source: TemplateSource,
    pub mapping: M,
}
pub type DataSourceTemplate = BaseDataSourceTemplate<Mapping>;

#[derive(Clone, Debug)]
pub struct DataSourceTemplateInfo<C: Blockchain> {
    pub template: C::DataSourceTemplate,
    pub params: Vec<String>,
    pub context: Option<DataSourceContext>,
    pub creation_block: BlockNumber,
}
 */
/*
/// Runtime representation of a data source.
// Note: Not great for memory usage that this needs to be `Clone`, considering how there may be tens
// of thousands of data sources in memory at once.
#[derive(Clone, Debug)]
pub struct DataSource {
    pub kind: String,
    pub network: Option<String>,
    pub name: String,
    pub source: Source,
    pub mapping: Mapping,
    pub context: Arc<Option<DataSourceContext>>,
    pub creation_block: Option<BlockNumber>,
    pub contract_abi: Arc<MappingABI>,
}
 */

impl FromValue<serde_yaml::Value, DataSource> for DataSource {
    type Error = anyhow::Error;
    /*
    pub fn from_manifest(manifest: &serde_yaml::Value) -> Vec<DataSource> {
        let mut res: Vec<DataSource> = Vec::default();
        if let Some(seqs) = manifest["dataSources"].as_sequence() {
            let datasources = seqs
                .iter()
                .map(|datasource| DataSource::from_value(datasource))
                .collect::<Vec<DataSource>>();
            res.extend(datasources);
        }
        res
    }
     */
    fn try_from(value: &serde_yaml::Value) -> Result<DataSource, Self::Error> {
        let datasource = DataSource {
            kind: value["kind"].as_str().unwrap().to_string(),
            network: None,
            name: value["name"].as_str().unwrap().to_string(),
            source: Source::try_from(&value["source"])?,
            mapping: Mapping::try_from(&value["mapping"])?,
            context: Arc::new(None),
            creation_block: None,
            contract_abi: Arc::new(MappingABI {
                name: "".to_string(),
                contract: Contract {
                    constructor: None,
                    functions: Default::default(),
                    events: Default::default(),
                    receive: false,
                    fallback: false,
                },
            }),
        };
        Ok(datasource)
    }
}
impl FromValue<serde_yaml::Value, DataSourceTemplate> for DataSourceTemplate {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, anyhow::Error> {
        let network = match &value["network"] {
            Value::String(val) => Some(val.clone()),
            _ => None,
        };
        //TemplateSource
        let map: &Value = &value["source"];
        let source = TemplateSource {
            abi: map["abi"].as_str().unwrap_or("").to_string(),
        };

        let map = &value["mapping"];

        let block_handlers = match map["blockHandlers"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|val| MappingBlockHandler {
                    handler: val["handler"].as_str().unwrap().to_string(),
                    filter: None,
                })
                .collect::<Vec<MappingBlockHandler>>(),
            _ => Vec::default(),
        };
        let call_handlers = match map["callHandlers"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|val| MappingCallHandler {
                    function: val["function"].as_str().unwrap().to_string(),
                    handler: val["handler"].as_str().unwrap().to_string(),
                })
                .collect::<Vec<MappingCallHandler>>(),
            _ => Vec::default(),
        };
        let event_handlers = match map["eventHandlers"].as_sequence() {
            Some(seqs) => seqs
                .iter()
                .map(|val| MappingEventHandler {
                    event: val["event"].as_str().unwrap().to_string(),
                    topic0: None,
                    handler: val["handler"].as_str().unwrap().to_string(),
                })
                .collect::<Vec<MappingEventHandler>>(),
            _ => Vec::default(),
        };
        let mapping = Mapping {
            kind: map["kind"].as_str().unwrap_or("").to_string(),
            api_version: Version::new(0, 0, 4),
            language: map["language"].as_str().unwrap_or("rust").to_string(),
            entities: vec![],
            abis: vec![],
            block_handlers,
            call_handlers,
            event_handlers,
            runtime: Arc::new(vec![]),
            link: Default::default(),
        };
        Ok(DataSourceTemplate {
            kind: value["kind"].as_str().unwrap_or("").to_string(),
            network,
            name: value["name"].as_str().unwrap_or("").to_string(),
            source,
            mapping,
        })
    }
}
/*
impl TryFrom<DataSourceTemplateInfo<Chain>> for DataSource {
    type Error = anyhow::Error;

    fn try_from(info: DataSourceTemplateInfo<Chain>) -> Result<Self, anyhow::Error> {
        let DataSourceTemplateInfo {
            template,
            params,
            context,
            creation_block,
        } = info;

        // Obtain the address from the parameters
        let string = params
            .get(0)
            .with_context(|| {
                format!(
                    "Failed to create data source from template `{}`: address parameter is missing",
                    template.name
                )
            })?
            .trim_start_matches("0x");

        let address = Address::from_str(string).with_context(|| {
            format!(
                "Failed to create data source from template `{}`, invalid address provided",
                template.name
            )
        })?;

        let contract_abi = template
            .mapping
            .find_abi(&template.source.abi)
            .with_context(|| format!("template `{}`", template.name))?;

        Ok(DataSource {
            kind: template.kind,
            network: template.network,
            name: template.name,
            source: Source {
                address: Some(address),
                abi: template.source.abi,
                start_block: 0,
            },
            mapping: template.mapping,
            context: Arc::new(context),
            creation_block: Some(creation_block),
            contract_abi,
        })
    }
}
*/
/*
impl blockchain::DataSource<Chain> for DataSource {
    fn mapping(&self) -> &crate::indexer::manifest::Mapping {
        todo!()
    }

    fn address(&self) -> Option<&[u8]> {
        todo!()
    }

    fn start_block(&self) -> BlockNumber {
        todo!()
    }

    fn from_manifest(
        kind: String,
        network: Option<String>,
        name: String,
        source: crate::indexer::manifest::Source,
        mapping: crate::indexer::manifest::Mapping,
        context: Option<crate::indexer::manifest::DataSourceContext>,
    ) -> Result<Self, Error> {
        todo!()
    }

    fn name(&self) -> &str {
        todo!()
    }

    fn kind(&self) -> &str {
        todo!()
    }

    fn network(&self) -> Option<&str> {
        todo!()
    }

    fn context(&self) -> Arc<Option<crate::indexer::manifest::DataSourceContext>> {
        todo!()
    }

    fn creation_block(&self) -> Option<BlockNumber> {
        todo!()
    }

    fn is_duplicate_of(&self, other: &Self) -> bool {
        todo!()
    }

    fn as_stored_dynamic_data_source(&self) -> StoredDynamicDataSource {
        todo!()
    }

    fn from_stored_dynamic_data_source(
        templates: &BTreeMap<&str, &crate::indexer::blockchain::DataSourceTemplate>,
        stored: StoredDynamicDataSource,
    ) -> Result<Self, Error> {
        todo!()
    }
}
 */
/// Deserialize an Address (with or without '0x' prefix).
fn deserialize_address<'de, D>(deserializer: D) -> Result<Option<Address>, D::Error>
where
    D: de::Deserializer<'de>,
{
    use serde::de::Error;

    let s: String = de::Deserialize::deserialize(deserializer)?;
    let address = s.trim_start_matches("0x");
    Address::from_str(address)
        .map_err(D::Error::custom)
        .map(Some)
}
