pub mod blockchain;
pub mod manifest;
pub mod types;

use crate::graph::cheap_clone::CheapClone;
use crate::graph::components::store::{EntityCache, EntityKey, WritableStore};
use crate::graph::data::store::Entity;
use crate::graph::runtime::{AscHeap, AscPtr, DeterministicHostError};
use crate::graph::util::lfu_cache::LfuCache;
use crate::indexer::blockchain::Blockchain;
use crate::indexer::manifest::{DataSourceTemplateInfo, Link};
use crate::prelude::slog::SendSyncRefUnwindSafeKV;
use crate::prelude::{impl_slog_value, Arc, Deserialize, Serialize, Version};
use anyhow::{anyhow, ensure, Error};
use lazy_static::lazy_static;
use serde::de;
use serde::ser;
use serde_yaml;
use slog::{debug, info, Logger};
use stable_hash::prelude::*;
use std::fmt;
use std::ops::Deref;

lazy_static! {
    static ref MAX_API_VERSION: Version = std::env::var("GRAPH_MAX_API_VERSION")
        .ok()
        .and_then(|api_version_str| Version::parse(&api_version_str).ok())
        .unwrap_or(Version::new(0, 0, 4));
}

pub struct IndexerState<C: Blockchain> {
    pub entity_cache: EntityCache,
    created_data_sources: Vec<DataSourceTemplateInfo<C>>,

    // Data sources created in the current handler.
    handler_created_data_sources: Vec<DataSourceTemplateInfo<C>>,

    // Marks whether a handler is currently executing.
    in_handler: bool,
}
impl<C: Blockchain> IndexerState<C> {
    pub fn new(
        store: Arc<dyn WritableStore>,
        lfu_cache: LfuCache<EntityKey, Option<Entity>>,
    ) -> Self {
        IndexerState {
            entity_cache: EntityCache::with_current(store, lfu_cache),
            created_data_sources: Vec::new(),
            handler_created_data_sources: Vec::new(),
            in_handler: false,
        }
    }
    pub fn enter_handler(&mut self) {}
    pub fn exit_handler(&mut self) {}
    pub fn exit_handler_and_discard_changes_due_to_error(&mut self) {}
    pub fn push_created_data_source(&mut self, ds: DataSourceTemplateInfo<C>) {
        assert!(self.in_handler);
        self.handler_created_data_sources.push(ds);
    }
}

// Note: This has a StableHash impl. Do not modify fields without a backward
// compatible change to the StableHash impl (below)
/// The IPFS hash used to identifiy a deployment externally, i.e., the
/// `Qm..` string that `graph-cli` prints when deploying to a subgraph
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeploymentHash(String);

impl StableHash for DeploymentHash {
    #[inline]
    fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
        self.0.stable_hash(sequence_number.next_child(), state);
    }
}

impl_slog_value!(DeploymentHash);

/// `DeploymentHash` is fixed-length so cheap to clone.
impl CheapClone for DeploymentHash {}

impl DeploymentHash {
    /// Check that `s` is a valid `SubgraphDeploymentId` and create a new one.
    /// If `s` is longer than 46 characters, or contains characters other than
    /// alphanumeric characters or `_`, return s (as a `String`) as the error
    pub fn new(s: impl Into<String>) -> Result<Self, String> {
        let s = s.into();

        // Enforce length limit
        if s.len() > 46 {
            return Err(s);
        }

        // Check that the ID contains only allowed characters.
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(s);
        }

        // Allow only deployment id's for 'real' subgraphs, not the old
        // metadata subgraph.
        if s == "subgraphs" {
            return Err(s);
        }

        Ok(DeploymentHash(s))
    }

    pub fn to_ipfs_link(&self) -> Link {
        Link {
            link: format!("/ipfs/{}", self),
        }
    }
}

impl Deref for DeploymentHash {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for DeploymentHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl ser::Serialize for DeploymentHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> de::Deserialize<'de> for DeploymentHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = de::Deserialize::deserialize(deserializer)?;
        DeploymentHash::new(s)
            .map_err(|s| de::Error::invalid_value(de::Unexpected::Str(&s), &"valid subgraph name"))
    }
}
/*
impl TryFromValue for DeploymentHash {
    fn try_from_value(value: &q::Value) -> Result<Self, Error> {
        Self::new(String::try_from_value(value)?)
            .map_err(|s| anyhow!("Invalid subgraph ID `{}`", s))
    }
}
 */
