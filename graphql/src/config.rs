use crate::CLEANUP_BLOCKS;
use http::HeaderMap;
use massbit_common::prelude::anyhow::{anyhow, bail, Context, Result};
use massbit_common::prelude::serde_json;
use massbit_common::prelude::slog::{info, Logger};
use massbit_data::indexer::NodeId;
use massbit_storage_postgres::indexer_store::Shard as ShardName;
use massbit_storage_postgres::{DeploymentPlacer, PRIMARY_SHARD};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::read_to_string;
use url::Url;
const ANY_NAME: &str = ".*";
/// A regular expression that matches nothing
const NO_NAME: &str = ".^";

#[derive(Debug)]
pub struct Opt {
    pub postgres_url: Option<String>,
    pub config: Option<String>,
    // This is only used when we construct a config purely from command
    // line options. When using a configuration file, pool sizes must be
    // set in the configuration file alone
    pub connection_pool_size: u32,
    pub postgres_secondary_hosts: Vec<String>,
    pub postgres_host_weights: Vec<usize>,
    pub node_id: String,
    pub http_port: u16,
    pub ws_port: u16,
    pub debug: bool,
}

impl Default for Opt {
    fn default() -> Self {
        Opt {
            postgres_url: None,
            config: None,
            connection_pool_size: 10,
            postgres_secondary_hosts: vec![],
            postgres_host_weights: vec![],
            node_id: "default".to_string(),
            http_port: 0,
            ws_port: 0,
            debug: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub general: Option<GeneralSection>,
    #[serde(rename = "store")]
    pub stores: BTreeMap<String, Shard>,
    pub deployment: Deployment,
}

fn validate_name(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(anyhow!("names must not be empty"));
    }
    if s.len() > 30 {
        return Err(anyhow!(
            "names can be at most 30 characters, but `{}` has {} characters",
            s,
            s.len()
        ));
    }

    if !s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(anyhow!(
            "name `{}` is invalid: names can only contain lowercase alphanumeric characters or '-'",
            s
        ));
    }
    Ok(())
}

impl Config {
    /// Check that the config is valid.
    fn validate(&mut self) -> Result<()> {
        if !self.stores.contains_key(PRIMARY_SHARD.as_str()) {
            return Err(anyhow!("missing a primary store"));
        }
        if self.stores.len() > 1 && *CLEANUP_BLOCKS {
            // See 8b6ad0c64e244023ac20ced7897fe666
            return Err(anyhow!(
                "GRAPH_ETHEREUM_CLEANUP_BLOCKS can not be used with a sharded store"
            ));
        }
        for (key, shard) in self.stores.iter_mut() {
            shard.validate(&key)?;
        }
        self.deployment.validate()?;

        // Check that deployment rules only reference existing stores and chains
        for (i, rule) in self.deployment.rules.iter().enumerate() {
            if !self.stores.contains_key(&rule.shard) {
                return Err(anyhow!(
                    "unknown shard {} in deployment rule {}",
                    rule.shard,
                    i
                ));
            }
        }
        Ok(())
    }

    /// Load a configuration file if `opt.config` is set. If not, generate
    /// a config from the command line arguments in `opt`
    pub fn load(logger: &Logger, opt: &Opt) -> Result<Config> {
        if let Some(config) = &opt.config {
            info!(logger, "Reading configuration file `{}`", config);
            let config = read_to_string(config)?;
            let mut config: Config = toml::from_str(&config)?;
            config.validate()?;
            Ok(config)
        } else {
            info!(
                logger,
                "Generating configuration from command line arguments"
            );
            Self::from_opt(opt)
        }
    }

    fn from_opt(opt: &Opt) -> Result<Config> {
        let deployment = Deployment::from_opt(opt);
        let mut stores = BTreeMap::new();
        stores.insert(PRIMARY_SHARD.to_string(), Shard::from_opt(opt)?);
        Ok(Config {
            general: None,
            stores,
            deployment,
        })
    }

    /// Genrate a JSON representation of the config.
    pub fn to_json(&self) -> Result<String> {
        // It would be nice to produce a TOML representation, but that runs
        // into this error: https://github.com/alexcrichton/toml-rs/issues/142
        // and fixing it as described in the issue didn't fix it. Since serializing
        // this data isn't crucial and only needed for debugging, we'll
        // just stick with JSON
        Ok(serde_json::to_string_pretty(&self)?)
    }

    pub fn primary_store(&self) -> &Shard {
        self.stores
            .get(PRIMARY_SHARD.as_str())
            .expect("a validated config has a primary store")
    }

    pub fn query_only(&self, node: &NodeId) -> bool {
        self.general
            .as_ref()
            .map(|g| match g.query.find(node.as_str()) {
                None => false,
                Some(m) => m.as_str() == node.as_str(),
            })
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GeneralSection {
    #[serde(with = "serde_regex", default = "no_name")]
    query: Regex,
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
        let pool_size = PoolSize::Fixed(opt.connection_pool_size);
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

    pub fn size_for(&self, node: &NodeId, name: &str) -> Result<u32> {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Replica {
    pub connection: String,
    #[serde(default = "one")]
    pub weight: usize,
    #[serde(default)]
    pub pool_size: PoolSize,
}

impl Replica {
    fn validate(&mut self, pool_size: &PoolSize) -> Result<()> {
        self.connection = shellexpand::env(&self.connection)?.into_owned();
        if matches!(self.pool_size, PoolSize::None) {
            self.pool_size = pool_size.clone();
        }

        self.pool_size.validate(&self.connection)?;
        Ok(())
    }
}

fn deserialize_http_headers<'de, D>(deserializer: D) -> Result<HeaderMap, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let kvs: BTreeMap<String, String> = Deserialize::deserialize(deserializer)?;
    Ok(btree_map_to_http_headers(kvs))
}

fn btree_map_to_http_headers(kvs: BTreeMap<String, String>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (k, v) in kvs.into_iter() {
        headers.insert(
            k.parse::<http::header::HeaderName>()
                .expect(&format!("invalid HTTP header name: {}", k)),
            v.parse::<http::header::HeaderValue>()
                .expect(&format!("invalid HTTP header value: {}: {}", k, v)),
        );
    }
    headers
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Transport {
    #[serde(rename = "rpc")]
    Rpc,
    #[serde(rename = "ws")]
    Ws,
    #[serde(rename = "ipc")]
    Ipc,
}

impl Default for Transport {
    fn default() -> Self {
        Self::Rpc
    }
}

impl std::fmt::Display for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Transport::*;

        match self {
            Rpc => write!(f, "rpc"),
            Ws => write!(f, "ws"),
            Ipc => write!(f, "ipc"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Deployment {
    #[serde(rename = "rule")]
    rules: Vec<Rule>,
}

impl Deployment {
    fn validate(&self) -> Result<()> {
        if self.rules.is_empty() {
            return Err(anyhow!(
                "there must be at least one deployment rule".to_string()
            ));
        }
        let mut default_rule = false;
        for rule in &self.rules {
            rule.validate()?;
            if default_rule {
                return Err(anyhow!("rules after a default rule are useless"));
            }
            default_rule = rule.is_default();
        }
        if !default_rule {
            return Err(anyhow!(
                "the rules do not contain a default rule that matches everything"
            ));
        }
        Ok(())
    }

    fn from_opt(_: &Opt) -> Self {
        Self { rules: vec![] }
    }
}

impl DeploymentPlacer for Deployment {
    fn place(&self, name: &str, network: &str) -> Result<Option<(ShardName, Vec<NodeId>)>, String> {
        // Errors here are really programming errors. We should have validated
        // everything already so that the various conversions can't fail. We
        // still return errors so that they bubble up to the deployment request
        // rather than crashing the node and burying the crash in the logs
        let placement = match self.rules.iter().find(|rule| rule.matches(name, network)) {
            Some(rule) => {
                let shard = ShardName::new(rule.shard.clone()).map_err(|e| e.to_string())?;
                let indexers: Vec<_> = rule
                    .indexers
                    .iter()
                    .map(|idx| {
                        NodeId::new(idx.clone())
                            .map_err(|()| format!("{} is not a valid node name", idx))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some((shard, indexers))
            }
            None => None,
        };
        Ok(placement)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Rule {
    #[serde(rename = "match", default)]
    pred: Predicate,
    #[serde(default = "primary_store")]
    shard: String,
    indexers: Vec<String>,
}

impl Rule {
    fn is_default(&self) -> bool {
        self.pred.matches_anything()
    }

    fn matches(&self, name: &str, network: &str) -> bool {
        self.pred.matches(name, network)
    }

    fn validate(&self) -> Result<()> {
        if self.indexers.is_empty() {
            return Err(anyhow!("useless rule without indexers"));
        }
        for indexer in &self.indexers {
            NodeId::new(indexer).map_err(|()| anyhow!("invalid node id {}", &indexer))?;
        }
        ShardName::new(self.shard.clone())
            .map_err(|e| anyhow!("illegal name for store shard `{}`: {}", &self.shard, e))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Predicate {
    #[serde(with = "serde_regex", default = "any_name")]
    name: Regex,
    network: Option<NetworkPredicate>,
}

impl Predicate {
    fn matches_anything(&self) -> bool {
        self.name.as_str() == ANY_NAME && self.network.is_none()
    }

    pub fn matches(&self, name: &str, network: &str) -> bool {
        if let Some(n) = &self.network {
            if !n.matches(network) {
                return false;
            }
        }

        match self.name.find(name) {
            None => false,
            Some(m) => m.as_str() == name,
        }
    }
}

impl Default for Predicate {
    fn default() -> Self {
        Predicate {
            name: any_name(),
            network: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum NetworkPredicate {
    Single(String),
    Many(Vec<String>),
}

impl NetworkPredicate {
    fn matches(&self, network: &str) -> bool {
        use NetworkPredicate::*;
        match self {
            Single(n) => n == network,
            Many(ns) => ns.iter().any(|n| n == network),
        }
    }

    fn to_vec(&self) -> Vec<String> {
        use NetworkPredicate::*;
        match self {
            Single(n) => vec![n.clone()],
            Many(ns) => ns.clone(),
        }
    }
}

/// Replace the host portion of `url` and return a new URL with `host`
/// as the host portion
///
/// Panics if `url` is not a valid URL (which won't happen in our case since
/// we would have paniced before getting here as `url` is the connection for
/// the primary Postgres instance)
fn replace_host(url: &str, host: &str) -> String {
    let mut url = match Url::parse(url) {
        Ok(url) => url,
        Err(_) => panic!("Invalid Postgres URL {}", url),
    };
    if let Err(e) = url.set_host(Some(host)) {
        panic!("Invalid Postgres url {}: {}", url, e.to_string());
    }
    String::from(url)
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

#[cfg(test)]
mod tests {

    use super::{Config, FirehoseProvider, Provider, ProviderDetails, Transport, Web3Provider};
    use http::{HeaderMap, HeaderValue};
    use std::collections::BTreeSet;
    use std::fs::read_to_string;
    use std::path::{Path, PathBuf};

    #[test]
    fn it_works_on_standard_config() {
        let content = read_resource_as_string("full_config.toml");
        let actual: Config = toml::from_str(&content).unwrap();

        // We do basic checks because writing the full equality method is really too long

        assert_eq!(
            "query_node_.*".to_string(),
            actual.general.unwrap().query.to_string()
        );
        assert_eq!(4, actual.chains.chains.len());
        assert_eq!(2, actual.stores.len());
        assert_eq!(3, actual.deployment.rules.len());
    }

    #[test]
    fn it_works_on_deprecated_provider_from_toml() {
        let actual = toml::from_str(
            r#"
            transport = "rpc"
            label = "peering"
            url = "http://localhost:8545"
            features = []
        "#,
        )
        .unwrap();

        assert_eq!(
            Provider {
                label: "peering".to_owned(),
                details: ProviderDetails::Web3(Web3Provider {
                    transport: Transport::Rpc,
                    url: "http://localhost:8545".to_owned(),
                    features: BTreeSet::new(),
                    headers: HeaderMap::new(),
                }),
            },
            actual
        );
    }

    #[test]
    fn it_works_on_deprecated_provider_without_transport_from_toml() {
        let actual = toml::from_str(
            r#"
            label = "peering"
            url = "http://localhost:8545"
            features = []
        "#,
        )
        .unwrap();

        assert_eq!(
            Provider {
                label: "peering".to_owned(),
                details: ProviderDetails::Web3(Web3Provider {
                    transport: Transport::Rpc,
                    url: "http://localhost:8545".to_owned(),
                    features: BTreeSet::new(),
                    headers: HeaderMap::new(),
                }),
            },
            actual
        );
    }

    #[test]
    fn it_errors_on_deprecated_provider_missing_url_from_toml() {
        let actual = toml::from_str::<Provider>(
            r#"
            transport = "rpc"
            label = "peering"
            features = []
        "#,
        );

        assert_eq!(true, actual.is_err());
        assert_eq!(
            actual.unwrap_err().to_string(),
            "missing field `url` at line 1 column 1"
        );
    }

    #[test]
    fn it_errors_on_deprecated_provider_missing_features_from_toml() {
        let actual = toml::from_str::<Provider>(
            r#"
            transport = "rpc"
            url = "http://localhost:8545"
            label = "peering"
        "#,
        );

        assert_eq!(true, actual.is_err());
        assert_eq!(
            actual.unwrap_err().to_string(),
            "missing field `features` at line 1 column 1"
        );
    }

    #[test]
    fn it_works_on_new_web3_provider_from_toml() {
        let actual = toml::from_str(
            r#"
            label = "peering"
            details = { type = "web3", transport = "ipc", url = "http://localhost:8545", features = ["archive"], headers = { x-test = "value" } }
        "#,
        )
            .unwrap();

        let mut features = BTreeSet::new();
        features.insert("archive".to_string());

        let mut headers = HeaderMap::new();
        headers.insert("x-test", HeaderValue::from_static("value"));

        assert_eq!(
            Provider {
                label: "peering".to_owned(),
                details: ProviderDetails::Web3(Web3Provider {
                    transport: Transport::Ipc,
                    url: "http://localhost:8545".to_owned(),
                    features,
                    headers,
                }),
            },
            actual
        );
    }

    #[test]
    fn it_works_on_new_web3_provider_without_transport_from_toml() {
        let actual = toml::from_str(
            r#"
            label = "peering"
            details = { type = "web3", url = "http://localhost:8545", features = [] }
        "#,
        )
        .unwrap();

        assert_eq!(
            Provider {
                label: "peering".to_owned(),
                details: ProviderDetails::Web3(Web3Provider {
                    transport: Transport::Rpc,
                    url: "http://localhost:8545".to_owned(),
                    features: BTreeSet::new(),
                    headers: HeaderMap::new(),
                }),
            },
            actual
        );
    }

    #[test]
    fn it_errors_on_new_provider_with_deprecated_fields_from_toml() {
        let actual = toml::from_str::<Provider>(
            r#"
            label = "peering"
            url = "http://localhost:8545"
            details = { type = "web3", url = "http://localhost:8545", features = [] }
        "#,
        );

        assert_eq!(true, actual.is_err());
        assert_eq!(actual.unwrap_err().to_string(), "when `details` field is provided, deprecated `url`, `transport`, `features` and `headers` cannot be specified at line 1 column 1");
    }

    #[test]
    fn it_works_on_new_firehose_provider_from_toml() {
        let actual = toml::from_str(
            r#"
                label = "firehose"
                details = { type = "firehose", url = "http://localhost:9000" }
            "#,
        )
        .unwrap();

        assert_eq!(
            Provider {
                label: "firehose".to_owned(),
                details: ProviderDetails::Firehose(FirehoseProvider {
                    url: "http://localhost:9000".to_owned(),
                }),
            },
            actual
        );
    }

    fn read_resource_as_string<P: AsRef<Path>>(path: P) -> String {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests");
        d.push(path);

        read_to_string(&d).expect(&format!("resource {:?} not found", &d))
    }
}
