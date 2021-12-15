use crate::entity::Entity;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;

/// The type we use for block numbers. This has to be a signed integer type
/// since Postgres does not support unsigned integer types. But 2G ought to
/// be enough for everybody
pub type BlockSlot = i64;

pub const BLOCK_NUMBER_MAX: BlockSlot = i64::MAX;

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

impl EntityKey {
    pub fn data(indexer_id: String, entity_type: String, entity_id: String) -> Self {
        Self {
            indexer_id,
            entity_type,
            entity_id,
        }
    }
}
