use crate::prelude::{q, IntoValue, TryFromValue};
use crate::store::Value;
use massbit_common::prelude::anyhow::{self, anyhow, Error};
use serde::{de, ser};
use serde_derive::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

pub mod deployment;
pub mod error;
use crate::indexer::error::IndexerError;
use crate::object;
use crate::store::scalar::Bytes;
pub use deployment::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IndexerName(String);

impl IndexerName {
    pub fn new(s: impl Into<String>) -> Result<Self, ()> {
        let s = s.into();

        // Note: these validation rules must be kept consistent with the validation rules
        // implemented in any other components that rely on indexer names.

        // Enforce length limits
        if s.is_empty() || s.len() > 255 {
            return Err(());
        }

        // Check that the name contains only allowed characters.
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/')
        {
            return Err(());
        }

        // Parse into components and validate each
        for part in s.split('/') {
            // Each part must be non-empty and not too long
            if part.is_empty() || part.len() > 32 {
                return Err(());
            }

            // To keep URLs unambiguous, reserve the token "graphql"
            if part == "graphql" {
                return Err(());
            }

            // Part should not start or end with a special character.
            let first_char = part.chars().next().unwrap();
            let last_char = part.chars().last().unwrap();
            if !first_char.is_ascii_alphanumeric()
                || !last_char.is_ascii_alphanumeric()
                || !part.chars().any(|c| c.is_ascii_alphabetic())
            {
                return Err(());
            }
        }

        Ok(IndexerName(s))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for IndexerName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl ser::Serialize for IndexerName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> de::Deserialize<'de> for IndexerName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s: String = de::Deserialize::deserialize(deserializer)?;
        IndexerName::new(s.clone())
            .map_err(|()| de::Error::invalid_value(de::Unexpected::Str(&s), &"valid indexer name"))
    }
}

#[derive(Debug, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
pub enum IndexerFeature {
    nonFatalErrors,
}

impl std::fmt::Display for IndexerFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexerFeature::nonFatalErrors => write!(f, "nonFatalErrors"),
        }
    }
}

impl FromStr for IndexerFeature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "nonFatalErrors" => Ok(IndexerFeature::nonFatalErrors),
            _ => Err(anyhow::anyhow!("invalid subgraph feature {}", s)),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum IndexerHealth {
    /// Syncing without errors.
    Healthy,

    /// Syncing but has errors.
    Unhealthy,

    /// No longer syncing due to fatal error.
    Failed,
}

impl IndexerHealth {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexerHealth::Healthy => "healthy",
            IndexerHealth::Unhealthy => "unhealthy",
            IndexerHealth::Failed => "failed",
        }
    }

    pub fn is_failed(&self) -> bool {
        match self {
            IndexerHealth::Healthy => false,
            IndexerHealth::Unhealthy => false,
            IndexerHealth::Failed => true,
        }
    }
}

impl FromStr for IndexerHealth {
    type Err = Error;

    fn from_str(s: &str) -> Result<IndexerHealth, Error> {
        match s {
            "healthy" => Ok(IndexerHealth::Healthy),
            "unhealthy" => Ok(IndexerHealth::Unhealthy),
            "failed" => Ok(IndexerHealth::Failed),
            _ => Err(anyhow!("failed to parse `{}` as SubgraphHealth", s)),
        }
    }
}

impl From<IndexerHealth> for String {
    fn from(health: IndexerHealth) -> String {
        health.as_str().to_string()
    }
}

impl From<IndexerHealth> for Value {
    fn from(health: IndexerHealth) -> Value {
        String::from(health).into()
    }
}

impl From<IndexerHealth> for q::Value {
    fn from(health: IndexerHealth) -> q::Value {
        q::Value::Enum(health.into())
    }
}

impl TryFromValue for IndexerHealth {
    fn try_from_value(value: &q::Value) -> Result<IndexerHealth, Error> {
        match value {
            q::Value::Enum(health) => IndexerHealth::from_str(health),
            _ => Err(anyhow!(
                "cannot parse value as SubgraphHealth: `{:?}`",
                value
            )),
        }
    }
}

// #[derive(Debug)]
// pub struct Info {
//     pub id: DeploymentId,
//
//     /// The deployment hash
//     pub indexer: String,
//
//     /// Whether or not the subgraph has synced all the way to the current chain head.
//     pub synced: bool,
//     pub health: IndexerHealth,
//     pub fatal_error: Option<IndexerError>,
//     pub non_fatal_errors: Vec<IndexerError>,
//
//     pub entity_count: u64,
//
//     /// ID of the Graph Node that the subgraph is indexed by.
//     pub node: Option<String>,
// }
//
// impl IntoValue for Info {
//     fn into_value(self) -> q::Value {
//         let Info {
//             id: _,
//             indexer,
//             entity_count,
//             fatal_error,
//             health,
//             node,
//             non_fatal_errors,
//             synced,
//         } = self;
//
//         fn subgraph_error_to_value(subgraph_error: IndexerError) -> q::Value {
//             let IndexerError {
//                 subgraph_id,
//                 message,
//                 block_ptr,
//                 handler,
//                 deterministic,
//             } = subgraph_error;
//
//             object! {
//                 __typename: "IndexerError",
//                 subgraphId: subgraph_id.to_string(),
//                 message: message,
//                 handler: handler,
//                 block: object! {
//                     __typename: "Block",
//                     number: block_ptr.as_ref().map(|x| x.number),
//                     hash: block_ptr.map(|x| q::Value::from(Value::Bytes(x.hash.into()))),
//                 },
//                 deterministic: deterministic,
//             }
//         }
//
//         let non_fatal_errors: Vec<q::Value> = non_fatal_errors
//             .into_iter()
//             .map(subgraph_error_to_value)
//             .collect();
//         let fatal_error_val = fatal_error.map_or(q::Value::Null, subgraph_error_to_value);
//
//         object! {
//             __typename: "SubgraphIndexingStatus",
//             indexer: indexer,
//             synced: synced,
//             health: q::Value::from(health),
//             fatalError: fatal_error_val,
//             nonFatalErrors: non_fatal_errors,
//             entityCount: format!("{}", entity_count),
//             node: node,
//         }
//     }
// }
