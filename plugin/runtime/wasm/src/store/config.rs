use crate::store::model::PRIMARY_SHARD;
use crate::store::ShardName;
use massbit_common::prelude::{
    anyhow::{anyhow, bail, Result},
    regex::Regex,
    serde_regex,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const ANY_NAME: &str = ".*";
/// A regular expression that matches nothing
const NO_NAME: &str = ".^";

pub struct Opt {
    pub postgres_url: Option<String>,
    pub config: Option<String>,
    // This is only used when we cosntruct a config purely from command
    // line options. When using a configuration file, pool sizes must be
    // set in the configuration file alone
    pub store_connection_pool_size: u32,
    pub postgres_secondary_hosts: Vec<String>,
    pub postgres_host_weights: Vec<usize>,
    pub disable_block_ingestor: bool,
    pub node_id: String,
    pub ethereum_rpc: Vec<String>,
    pub ethereum_ws: Vec<String>,
    pub ethereum_ipc: Vec<String>,
}

impl Default for Opt {
    fn default() -> Self {
        Opt {
            postgres_url: None,
            config: None,
            store_connection_pool_size: 10,
            postgres_secondary_hosts: vec![],
            postgres_host_weights: vec![],
            disable_block_ingestor: true,
            node_id: "default".to_string(),
            ethereum_rpc: vec![],
            ethereum_ws: vec![],
            ethereum_ipc: vec![],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShardConfig {
    pub connection: String,
    #[serde(default = "one")]
    pub weight: usize,
    #[serde(default)]
    pub pool_size: PoolSize,
    #[serde(default = "PoolSize::five")]
    pub fdw_pool_size: PoolSize,
    //#[serde(default)]
    //pub replicas: BTreeMap<String, Replica>,
}

impl ShardConfig {
    /*
    fn validate(&mut self, name: &str) -> Result<()> {
        ShardName::new(name.to_string()).map_err(|e| anyhow!(e))?;

        self.connection = shellexpand::env(&self.connection)?.into_owned();

        if matches!(self.pool_size, PoolSize::None) {
            return Err(anyhow!("missing pool size definition for shard `{}`", name));
        }

        self.pool_size.validate(&self.connection)?;
        for (name, replica) in self.replicas.iter_mut() {
            validate_name(name).context("illegal replica name")?;
            replica.validate(&self.pool_size)?;
        }
        Ok(())
    }

    fn from_opt(opt: &Opt) -> Result<Self> {
        let postgres_url = opt
            .postgres_url
            .as_ref()
            .expect("validation checked that postgres_url is set");
        let pool_size = PoolSize::Fixed(opt.store_connection_pool_size);
        pool_size.validate(&postgres_url)?;
        let mut replicas = BTreeMap::new();
        for (i, host) in opt.postgres_secondary_hosts.iter().enumerate() {
            let replica = Replica {
                connection: replace_host(&postgres_url, &host),
                weight: opt.postgres_host_weights.get(i + 1).cloned().unwrap_or(1),
                pool_size: pool_size.clone(),
            };
            replicas.insert(format!("replica{}", i + 1), replica);
        }
        Ok(Self {
            connection: postgres_url.clone(),
            weight: opt.postgres_host_weights.get(0).cloned().unwrap_or(1),
            pool_size,
            fdw_pool_size: PoolSize::five(),
            replicas,
        })
    }
     */
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PoolSize {
    None,
    Fixed(u32),
    Rule(Vec<PoolSizeRule>),
}

impl Default for PoolSize {
    fn default() -> Self {
        Self::None
    }
}

impl PoolSize {
    fn five() -> Self {
        Self::Fixed(5)
    }

    fn validate(&self, connection: &str) -> Result<()> {
        use PoolSize::*;

        let pool_size = match self {
            None => bail!("missing pool size for {}", connection),
            Fixed(s) => s.clone(),
            Rule(rules) => rules.iter().map(|rule| rule.size).min().unwrap_or(0u32),
        };

        if pool_size < 2 {
            Err(anyhow!(
                "connection pool size must be at least 2, but is {} for {}",
                pool_size,
                connection
            ))
        } else {
            Ok(())
        }
    }

    pub fn size_for(&self, node: &String, name: &str) -> Result<u32> {
        use PoolSize::*;
        match self {
            None => unreachable!("validation ensures we have a pool size"),
            Fixed(s) => Ok(s.clone()),
            Rule(rules) => rules
                .iter()
                .find(|rule| rule.matches(node.as_str()))
                .map(|rule| rule.size)
                .ok_or_else(|| {
                    anyhow!(
                        "no rule matches `{}` for the pool of shard {}",
                        node.as_str(),
                        name
                    )
                }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PoolSizeRule {
    #[serde(with = "serde_regex", default = "any_name")]
    node: Regex,
    size: u32,
}

impl PoolSizeRule {
    fn matches(&self, name: &str) -> bool {
        match self.node.find(name) {
            None => false,
            Some(m) => m.as_str() == name,
        }
    }
}

// Various default functions for deserialization
fn any_name() -> Regex {
    Regex::new(ANY_NAME).unwrap()
}

fn no_name() -> Regex {
    Regex::new(NO_NAME).unwrap()
}

fn primary_store() -> String {
    PRIMARY_SHARD.to_string()
}

fn one() -> usize {
    1
}
