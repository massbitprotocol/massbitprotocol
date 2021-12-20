use crate::graphql::TryFromValue;
use crate::impl_slog_value;
use crate::prelude::q;
use crate::store::chain::{BlockNumber, BlockPtr};
use indexer_orm::models::DeploymentId as ModelDeploymentId;
use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::anyhow::{self, anyhow, Error};
use massbit_common::prelude::stable_hash::{SequenceNumber, StableHash, StableHasher};
use serde::{de, ser};
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(s: impl Into<String>) -> Result<Self, ()> {
        let s = s.into();

        // Enforce length limit
        if s.len() > 63 {
            return Err(());
        }

        // Check that the ID contains only allowed characters.
        // Note: these restrictions are relied upon to prevent SQL injection
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(());
        }

        Ok(NodeId(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl slog::Value for NodeId {
    fn serialize(
        &self,
        _record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_str(key, self.0.as_str())
    }
}

impl<'de> de::Deserialize<'de> for NodeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = de::Deserialize::deserialize(deserializer)?;
        NodeId::new(s.clone())
            .map_err(|()| de::Error::invalid_value(de::Unexpected::Str(&s), &"valid node ID"))
    }
}

/// An internal identifer for the specific instance of a deployment. The
/// identifier only has meaning in the context of a specific instance of
/// graph-node. Only store code should ever construct or consume it; all
/// other code passes it around as an opaque token.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DeploymentId(pub i32);

impl Display for DeploymentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl DeploymentId {
    pub fn new(id: i32) -> Self {
        Self(id)
    }
}

impl From<DeploymentId> for ModelDeploymentId {
    fn from(id: DeploymentId) -> Self {
        ModelDeploymentId::new(id.0)
    }
}

impl From<ModelDeploymentId> for DeploymentId {
    fn from(id: ModelDeploymentId) -> Self {
        DeploymentId(id.value())
    }
}
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
    /// Check that `s` is a valid `IndexerDeploymentId` and create a new one.
    /// If `s` is longer than 46 characters, or contains characters other than
    /// alphanumeric characters or `_`, return s (as a `String`) as the error
    pub fn new(s: impl Into<String>) -> Result<Self, String> {
        let s = s.into();

        // Enforce length limit
        if s.len() > 46 {
            return Err(s);
        }

        // Check that the ID contains only allowed characters.
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(s);
        }
        Ok(DeploymentHash(s))
    }

    // pub fn to_ipfs_link(&self) -> Link {
    //     Link {
    //         link: format!("/ipfs/{}", self),
    //     }
    // }
}

impl Deref for DeploymentHash {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for DeploymentHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
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

impl TryFromValue for DeploymentHash {
    fn try_from_value(value: &q::Value) -> Result<Self, Error> {
        Self::new(String::try_from_value(value)?)
            .map_err(|s| anyhow!("Invalid subgraph ID `{}`", s))
    }
}

/// A unique identifier for a deployment that specifies both its external
/// identifier (`hash`) and its unique internal identifier (`id`) which
/// ensures we are talking about a unique location for the deployment's data
/// in the store
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DeploymentLocator {
    pub id: DeploymentId,
    pub hash: DeploymentHash,
}

impl CheapClone for DeploymentLocator {}

impl slog::Value for DeploymentLocator {
    fn serialize(
        &self,
        record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        slog::Value::serialize(&self.to_string(), record, key, serializer)
    }
}

impl DeploymentLocator {
    pub fn new(id: DeploymentId, hash: DeploymentHash) -> Self {
        Self { id, hash }
    }
}

impl Display for DeploymentLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.hash, self.id)
    }
}

/// Important details about the current state of a subgraph deployment
/// used while executing queries against a deployment
///
/// The `reorg_count` and `max_reorg_depth` fields are maintained (in the
/// database) by `store::metadata::forward_block_ptr` and
/// `store::metadata::revert_block_ptr` which get called as part of transacting
/// new entities into the store or reverting blocks.
#[derive(Debug, Clone)]
pub struct DeploymentState {
    pub hash: DeploymentHash,
    /// The number of blocks that were ever reverted in this subgraph. This
    /// number increases monotonically every time a block is reverted
    pub reorg_count: u32,
    /// The maximum number of blocks we ever reorged without moving a block
    /// forward in between
    pub max_reorg_depth: u32,
    /// The number of the last block that the subgraph has processed
    pub latest_block_number: BlockNumber,
}

impl DeploymentState {
    /// Is this subgraph deployed and has it processed any blocks?
    pub fn is_deployed(&self) -> bool {
        self.latest_block_number > 0
    }
}

#[derive(Debug)]
pub struct IndexerDeploymentEntity {
    pub manifest: IndexerManifestEntity,
    pub earliest_block: Option<BlockPtr>,
    pub latest_block: Option<BlockPtr>,
}

#[derive(Debug)]
pub struct IndexerManifestEntity {
    pub spec_version: String,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub schema: String,
    pub features: Vec<String>,
}
