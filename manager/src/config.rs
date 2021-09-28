use massbit::prelude::{
    anyhow::{anyhow, bail, Context, Result},
    *,
};
use massbit_store_postgres::{Shard as ShardName, PRIMARY_SHARD};
use std::fs::read_to_string;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};
use url::Url;

pub struct Opt {
    pub postgres_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub stores: BTreeMap<String, Shard>,
}

impl Config {
    /// Load a configuration file if `opt.config` is set. If not, generate
    /// a config from the command line arguments in `opt`
    pub fn load(opt: &Opt) -> Result<Config> {
        info!("Generating configuration from command line arguments");
        Self::from_opt(opt)
    }

    fn from_opt(opt: &Opt) -> Result<Config> {
        let mut stores = BTreeMap::new();
        stores.insert(PRIMARY_SHARD.to_string(), Shard::from_opt(opt)?);
        Ok(Config { stores })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shard {
    pub connection: String,
    #[serde(default = "one")]
    pub weight: usize,
    #[serde(default)]
    pub pool_size: PoolSize,
    #[serde(default = "PoolSize::five")]
    pub fdw_pool_size: PoolSize,
    #[serde(default)]
    pub replicas: BTreeMap<String, Replica>,
}

impl Shard {
    fn from_opt(opt: &Opt) -> Result<Self> {
        let postgres_url = opt
            .postgres_url
            .as_ref()
            .expect("validation checked that postgres_url is set");
        let pool_size = PoolSize::Fixed(5);
        let mut replicas = BTreeMap::new();
        Ok(Self {
            connection: postgres_url.clone(),
            weight: 1,
            pool_size,
            fdw_pool_size: PoolSize::five(),
            replicas,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PoolSize {
    None,
    Fixed(u32),
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

    pub fn size_for(&self, name: &str) -> Result<u32> {
        use PoolSize::*;
        match self {
            None => unreachable!("validation ensures we have a pool size"),
            Fixed(s) => Ok(*s),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Replica {
    pub connection: String,
    #[serde(default = "one")]
    pub weight: usize,
    #[serde(default)]
    pub pool_size: PoolSize,
}

fn one() -> usize {
    1
}
