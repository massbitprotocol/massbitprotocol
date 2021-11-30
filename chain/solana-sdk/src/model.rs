use crate::entity::Entity;
use anyhow::{anyhow, Error};
use serde::{de, ser, Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fmt::Display;
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;

/// The type we use for block numbers. This has to be a signed integer type
/// since Postgres does not support unsigned integer types. But 2G ought to
/// be enough for everybody
pub type BlockSlot = i32;

pub const BLOCK_NUMBER_MAX: BlockSlot = i32::MAX;

/// An internal identifer for the specific instance of a deployment. The
/// identifier only has meaning in the context of a specific instance of
/// massbit. Only store code should ever construct or consume it; all
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

/// An entity operation that can be transacted into the store; as opposed to
/// `EntityOperation`, we already know whether a `Set` should be an `Insert`
/// or `Update`
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityModification {
    /// Insert the entity
    Insert { key: EntityKey, data: Entity },
    /// Update the entity by overwriting it
    Overwrite { key: EntityKey, data: Entity },
    /// Remove the entity
    Remove { key: EntityKey },
}

impl EntityModification {
    pub fn entity_key(&self) -> &EntityKey {
        use EntityModification::*;
        match self {
            Insert { key, .. } | Overwrite { key, .. } | Remove { key } => key,
        }
    }

    pub fn is_remove(&self) -> bool {
        match self {
            EntityModification::Remove { .. } => true,
            _ => false,
        }
    }
}

/// A representation of entity operations that can be accumulated.
#[derive(Debug, Clone)]
enum EntityOp {
    Remove,
    Update(Entity),
    Overwrite(Entity),
}

impl EntityOp {
    fn apply_to(self, entity: Option<Entity>) -> Option<Entity> {
        use EntityOp::*;
        match (self, entity) {
            (Remove, _) => None,
            (Overwrite(new), _) | (Update(new), None) => Some(new),
            (Update(updates), Some(mut entity)) => {
                entity.merge_remove_null_fields(updates);
                Some(entity)
            }
        }
    }

    fn accumulate(&mut self, next: EntityOp) {
        use EntityOp::*;
        let update = match next {
            // Remove and Overwrite ignore the current value.
            Remove | Overwrite(_) => {
                *self = next;
                return;
            }
            Update(update) => update,
        };

        // We have an update, apply it.
        match self {
            // This is how `Overwrite` is constructed, by accumulating `Update` onto `Remove`.
            Remove => *self = Overwrite(update),
            Update(current) | Overwrite(current) => current.merge(update),
        }
    }
}

/// An entity operation that can be transacted into the store.
#[derive(Clone, Debug, PartialEq)]
pub enum EntityOperation {
    /// Locates the entity specified by `key` and sets its attributes according to the contents of
    /// `data`.  If no entity exists with this key, creates a new entity.
    Set { key: EntityKey, data: Entity },

    /// Removes an entity with the specified key, if one exists.
    Remove { key: EntityKey },
}

// /// The type name of an entity. This is the string that is used in the
// /// indexer's GraphQL schema as `type NAME @entity { .. }`
// #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
// pub struct EntityType(String);
//
// impl EntityType {
//     /// Construct a new entity type. Ideally, this is only called when
//     /// `entity_type` either comes from the GraphQL schema, or from
//     /// the database from fields that are known to contain a valid entity type
//     pub fn new(entity_type: String) -> Self {
//         Self(entity_type)
//     }
//
//     pub fn as_str(&self) -> &str {
//         &self.0
//     }
//
//     pub fn into_string(self) -> String {
//         self.0
//     }
// }
//
// impl fmt::Display for EntityType {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }
//
// // This conversion should only be used in tests since it makes it too
// // easy to convert random strings into entity types
// #[cfg(debug_assertions)]
// impl From<&str> for EntityType {
//     fn from(s: &str) -> Self {
//         EntityType::new(s.to_owned())
//     }
// }

// Note: Do not modify fields without making a backward compatible change to
// the StableHash impl (below)
/// Key by which an individual entity in the store can be accessed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityKey {
    /// ID of the indexer.
    pub indexer_id: String, //Indexer hash

    /// Name of the entity type.
    pub entity_type: String,

    /// ID of the individual entity.
    pub entity_id: String,
}

// impl StableHash for EntityKey {
//     fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
//         self.indexer_id
//             .stable_hash(sequence_number.next_child(), state);
//         self.entity_type
//             .as_str()
//             .stable_hash(sequence_number.next_child(), state);
//         self.entity_id
//             .stable_hash(sequence_number.next_child(), state);
//     }
// }

impl EntityKey {
    pub fn data(indexer_id: String, entity_type: String, entity_id: String) -> Self {
        Self {
            indexer_id,
            entity_type,
            entity_id,
        }
    }
}

// #[derive(Error, Debug)]
// pub enum StoreError {
//     #[error("store error: {0}")]
//     Unknown(Error),
//     #[error(
//         "tried to set entity of type `{0}` with ID \"{1}\" but an entity of type `{2}`, \
//          which has an interface in common with `{0}`, exists with the same ID"
//     )]
//     ConflictingId(String, String, String), // (entity, id, conflicting_entity)
//     #[error("unknown field '{0}'")]
//     UnknownField(String),
//     #[error("unknown table '{0}'")]
//     UnknownTable(String),
//     #[error("malformed directive '{0}'")]
//     MalformedDirective(String),
//     #[error("query execution failed: {0}")]
//     QueryExecutionError(String),
//     #[error("invalid identifier: {0}")]
//     InvalidIdentifier(String),
//     #[error(
//         "indexer `{0}` has already processed block `{1}`; \
//          there are most likely two (or more) nodes indexing this indexer"
//     )]
//     DuplicateBlockProcessing(String, BlockSlot),
//     /// An internal error where we expected the application logic to enforce
//     /// some constraint, e.g., that indexer names are unique, but found that
//     /// constraint to not hold
//     #[error("internal constraint violated: {0}")]
//     ConstraintViolation(String),
//     #[error("deployment not found: {0}")]
//     DeploymentNotFound(String),
//     #[error("shard not found: {0} (this usually indicates a misconfiguration)")]
//     UnknownShard(String),
//     #[error("Fulltext search not yet deterministic")]
//     FulltextSearchNonDeterministic,
//     #[error("operation was canceled")]
//     Canceled,
//     #[error("database unavailable")]
//     DatabaseUnavailable,
// }
//
// // Convenience to report a constraint violation
// #[macro_export]
// macro_rules! constraint_violation {
//     ($msg:expr) => {{
//         StoreError::ConstraintViolation(format!("{}", $msg))
//     }};
//     ($fmt:expr, $($arg:tt)*) => {{
//         StoreError::ConstraintViolation(format!($fmt, $($arg)*))
//     }}
// }
//
// impl From<::diesel::result::Error> for StoreError {
//     fn from(e: ::diesel::result::Error) -> Self {
//         StoreError::Unknown(e.into())
//     }
// }
//
// impl From<::diesel::r2d2::PoolError> for StoreError {
//     fn from(e: ::diesel::r2d2::PoolError) -> Self {
//         StoreError::Unknown(e.into())
//     }
// }
//
// impl From<Error> for StoreError {
//     fn from(e: Error) -> Self {
//         StoreError::Unknown(e)
//     }
// }
//
// impl From<serde_json::Error> for StoreError {
//     fn from(e: serde_json::Error) -> Self {
//         StoreError::Unknown(e.into())
//     }
// }
//
// // impl From<QueryExecutionError> for StoreError {
// //     fn from(e: QueryExecutionError) -> Self {
// //         StoreError::QueryExecutionError(e.to_string())
// //     }
// // }
//
// impl From<std::fmt::Error> for StoreError {
//     fn from(e: std::fmt::Error) -> Self {
//         StoreError::Unknown(anyhow!("{}", e.to_string()))
//     }
// }
