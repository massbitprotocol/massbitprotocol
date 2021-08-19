use super::{Mapping, MappingABI};
use crate::chain::ethereum::Chain;
use crate::graph::data::store::Entity;
use crate::graph::prelude::BlockNumber;
use crate::indexer::blockchain::{self, Blockchain};
use crate::indexer::manifest::StoredDynamicDataSource;
use crate::prelude::anyhow::Context;
use crate::prelude::serde_yaml::Value;
use crate::prelude::Error;
use serde::de;
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::{convert::TryFrom, sync::Arc};
use web3::ethabi::Address;

pub type DataSourceContext = Entity;
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
impl Source {
    pub fn from_value(value: &serde_yaml::Value) -> Source {
        let address = match &value["address"] {
            Value::String(addr) => Some(Address::from_str(addr).with_context(|| {
                format!(
                    "Failed to create address from value `{}`, invalid address provided",
                    addr
                )
            })?),
            _ => None,
        };
        Source {
            address,
            abi: value["abi"].as_str().unwrap().to_string(),
            start_block: value["startBlock"].as_i64().unwrap() as i32,
        }
    }
}
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
impl DataSource {
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
    pub fn from_value(value: &serde_yaml::Value) -> DataSource {
        let datasource = DataSource {
            kind: value["kind"].as_str().unwrap().to_string(),
            network: None,
            name: value["name"].as_str().unwrap().to_string(),
            source: Source::from_value(&value["source"]),
            mapping: Mapping::from_value(&value["mapping"]),
            context: Arc::new(None),
            creation_block: None,
            contract_abi: Arc::new(MappingABI {
                name: "".to_string(),
                contract: Default::default(),
            }),
        };
        datasource
    }
}

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
