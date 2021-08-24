pub mod blockchain;
pub mod manifest;
pub mod types;

use crate::graph::cheap_clone::CheapClone;
use crate::graph::runtime::{AscHeap, AscPtr, DeterministicHostError};
use crate::indexer::blockchain::Blockchain;
use crate::indexer::manifest::{DataSourceTemplateInfo, Link};
use crate::prelude::slog::SendSyncRefUnwindSafeKV;
use crate::prelude::{impl_slog_value, Arc, Version};
use crate::store::{
    Entity, EntityCache, EntityKey, ModificationsAndCache, QueryExecutionError, StoreError,
    WritableStore,
};
use crate::util::lfu_cache::LfuCache;
use massbit_common::prelude::{
    anyhow::{anyhow, ensure, Error},
    lazy_static::lazy_static,
    serde::{de, ser},
    //Deserialize, Serialize},
    serde_yaml,
};

use slog::{debug, info, Logger};
use stable_hash::prelude::*;
use std::fmt;
use std::ops::Deref;
use std::sync::Mutex;

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

    /*
    pub fn extend(&mut self, other: IndexerState<C>) {
        assert!(!other.in_handler);

        let IndexerState {
            entity_cache,
            created_data_sources,
            handler_created_data_sources,
            in_handler,
        } = self;

        match in_handler {
            true => handler_created_data_sources.extend(other.created_data_sources),
            false => created_data_sources.extend(other.created_data_sources),
        }
        entity_cache
            .lock()
            .unwrap()
            .extend(*other.entity_cache.lock().unwrap());
    }
    */
    pub fn has_errors(&self) -> bool {
        false
    }

    pub fn has_created_data_sources(&self) -> bool {
        assert!(!self.in_handler);
        !self.created_data_sources.is_empty()
    }

    pub fn drain_created_data_sources(&mut self) -> Vec<DataSourceTemplateInfo<C>> {
        assert!(!self.in_handler);
        std::mem::replace(&mut self.created_data_sources, Vec::new())
    }

    pub fn enter_handler(&mut self) {
        assert!(!self.in_handler);
        self.in_handler = true;
        self.entity_cache.enter_handler()
    }

    pub fn exit_handler(&mut self) {
        assert!(self.in_handler);
        self.in_handler = false;
        self.created_data_sources
            .extend(self.handler_created_data_sources.drain(..));
        self.entity_cache.exit_handler()
    }

    pub fn exit_handler_and_discard_changes_due_to_error(&mut self) {
        assert!(self.in_handler);
        self.in_handler = false;
        self.handler_created_data_sources.clear();
        self.entity_cache.exit_handler_and_discard_changes();
        //self.deterministic_errors.push(e);
    }

    pub fn push_created_data_source(&mut self, ds: DataSourceTemplateInfo<C>) {
        assert!(self.in_handler);
        self.handler_created_data_sources.push(ds);
    }
    pub fn get_entity(&mut self, key: &EntityKey) -> Result<Option<Entity>, QueryExecutionError> {
        self.entity_cache.get(key)
    }
    pub fn set_entity(&mut self, key: EntityKey, entity: Entity) {
        self.entity_cache.set(key, entity);
    }
    pub fn remove_entity(&mut self, key: EntityKey) {
        self.entity_cache.remove(key);
    }
}
/*
impl<C: Blockchain> IndexerState<C> {
    pub fn flush_cache(&mut self) -> Result<(), QueryExecutionError> {
        assert!(self.in_handler);
        let ModificationsAndCache {
            modifications: mods,
            entity_lfu_cache: cache,
        } = self.entity_cache.lock().unwrap().as_modifications()?;
        //.map_err(|e| StoreError::Unknown(e.into()))?;
        Ok(())
        //.map_err(|e| BlockProcessingError::Unknown(e.into()))?;
    }
}
*/
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
