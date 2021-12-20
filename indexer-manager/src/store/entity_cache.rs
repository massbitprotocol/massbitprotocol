use crate::store::IndexerStoreTrait;
use core::fmt::Debug;
use massbit_common::prelude::anyhow;
use massbit_data::query::{CloneableAnyhowError, QueryExecutionError};
use massbit_data::store::{Entity, EntityKey, EntityModification, EntityType};
//use massbit_solana_sdk::model::{EntityModification, EntityOperation};
use massbit_data::store::entity::EntityOperation;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::iter::FromIterator;
use std::sync::Arc;

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
pub struct ModificationsAndCache {
    pub modifications: Vec<EntityModification>,
    pub entity_lfu_cache: HashMap<EntityKey, Option<Entity>>,
}
/// A cache for entities from the store that provides the basic functionality
/// needed for the store interactions in the host exports. This struct tracks
/// how entities are modified, and caches all entities looked up from the
/// store. The cache makes sure that
///   (1) no entity appears in more than one operation
///   (2) only entities that will actually be changed from what they
///       are in the store are changed
pub struct EntityCache {
    /// The state of entities in the store. An entry of `None`
    /// means that the entity is not present in the store
    current: HashMap<EntityKey, Option<Entity>>,

    /// The accumulated changes to an entity.
    updates: HashMap<EntityKey, EntityOp>,
    // Updates for a currently executing handler.
    handler_updates: HashMap<EntityKey, EntityOp>,

    // Marks whether updates should go in `handler_updates`.
    in_handler: bool,
    pub store: Arc<dyn IndexerStoreTrait>,
}
impl EntityCache {
    pub fn new(store: Arc<dyn IndexerStoreTrait>) -> Self {
        Self {
            current: HashMap::new(),
            updates: HashMap::new(),
            handler_updates: HashMap::new(),
            in_handler: false,
            store,
        }
    }
}

impl EntityCache {
    pub(crate) fn enter_handler(&mut self) {
        assert!(!self.in_handler);
        self.in_handler = true;
    }

    pub(crate) fn exit_handler(&mut self) {
        assert!(self.in_handler);
        self.in_handler = false;

        // Apply all handler updates to the main `updates`.
        let handler_updates = Vec::from_iter(self.handler_updates.drain());
        for (key, op) in handler_updates {
            self.entity_op(key, op)
        }
    }

    pub(crate) fn exit_handler_and_discard_changes(&mut self) {
        assert!(self.in_handler);
        self.in_handler = false;
        self.handler_updates.clear();
    }

    pub(crate) fn extend(&mut self, other: EntityCache) {
        assert!(!other.in_handler);

        self.current.extend(other.current);
        for (key, op) in other.updates {
            self.entity_op(key, op);
        }
    }
    pub fn get(&mut self, key: &EntityKey) -> Result<Option<Entity>, QueryExecutionError> {
        // Get the current entity, apply any updates from `updates`, then from `handler_updates`.
        let mut entity = self.get_current_entity(key)?;
        if let Some(op) = self.updates.get(key).cloned() {
            entity = op.apply_to(entity)
        }
        if let Some(op) = self.handler_updates.get(key).cloned() {
            entity = op.apply_to(entity)
        }
        Ok(entity)
    }
    fn get_current_entity(
        &mut self,
        key: &EntityKey,
    ) -> Result<Option<Entity>, QueryExecutionError> {
        match self.current.get(key) {
            None => {
                let mut entity = self.store.get(key)?;
                if let Some(entity) = &mut entity {
                    // `__typename` is for queries not for mappings.
                    entity.remove("__typename");
                }
                self.current.insert(key.clone(), entity.clone());
                Ok(entity)
            }
            Some(data) => Ok(data.to_owned()),
        }
    }
    pub fn remove(&mut self, key: EntityKey) {
        self.entity_op(key, EntityOp::Remove);
    }

    pub fn set(&mut self, key: EntityKey, entity: Entity) {
        self.entity_op(key, EntityOp::Update(entity))
    }

    pub fn append(&mut self, operations: Vec<EntityOperation>) {
        assert!(!self.in_handler);

        for operation in operations {
            match operation {
                EntityOperation::Set { key, data } => {
                    self.entity_op(key, EntityOp::Update(data));
                }
                EntityOperation::Remove { key } => {
                    self.entity_op(key, EntityOp::Remove);
                }
            }
        }
    }
    fn entity_op(&mut self, key: EntityKey, op: EntityOp) {
        use std::collections::hash_map::Entry;
        let updates = match self.in_handler {
            true => &mut self.handler_updates,
            false => &mut self.updates,
        };

        match updates.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(op);
            }
            Entry::Occupied(mut entry) => entry.get_mut().accumulate(op),
        }
    }
    /// Return the changes that have been made via `set` and `remove` as
    /// `EntityModification`, making sure to only produce one when a change
    /// to the current state is actually needed.
    ///
    /// Also returns the updated `LfuCache`.
    pub fn as_modifications(mut self) -> Result<ModificationsAndCache, QueryExecutionError> {
        assert!(!self.in_handler);

        // The first step is to make sure all entities being set are in `self.current`.
        // For each indexer, we need a map of entity type to missing entity ids.
        let missing = self
            .updates
            .keys()
            .filter(|key| !self.current.contains_key(key));

        let mut missing_by_indexer: BTreeMap<_, BTreeMap<&EntityType, Vec<&str>>> = BTreeMap::new();
        for key in missing {
            missing_by_indexer
                .entry(&key.indexer_hash)
                .or_default()
                .entry(&key.entity_type)
                .or_default()
                .push(&key.entity_id);
        }

        for (indexer_id, keys) in missing_by_indexer {
            for (entity_type, entities) in self.store.get_many(keys).map_err(|e| {
                let err: anyhow::Error = e.into();
                QueryExecutionError::StoreError(CloneableAnyhowError::from(err))
            })? {
                for entity in entities {
                    let key = EntityKey {
                        indexer_hash: indexer_id.clone(),
                        entity_type: entity_type.clone(),
                        entity_id: entity.id().unwrap(),
                    };
                    self.current.insert(key, Some(entity));
                }
            }
        }

        let mut mods = Vec::new();
        for (key, update) in self.updates {
            use EntityModification::*;
            let current = self.current.remove(&key).and_then(|entity| entity);
            let modification = match (current, update) {
                // Entity was created
                (None, EntityOp::Update(updates)) | (None, EntityOp::Overwrite(updates)) => {
                    // Merging with an empty entity removes null fields.
                    let mut data = Entity::new();
                    data.merge_remove_null_fields(updates);
                    self.current.insert(key.clone(), Some(data.clone()));
                    Some(Insert { key, data })
                }
                // Entity may have been changed
                (Some(current), EntityOp::Update(updates)) => {
                    let mut data = current.clone();
                    data.merge_remove_null_fields(updates);
                    self.current.insert(key.clone(), Some(data.clone()));
                    if current != data {
                        Some(Overwrite { key, data })
                    } else {
                        None
                    }
                }
                // Entity was removed and then updated, so it will be overwritten
                (Some(current), EntityOp::Overwrite(data)) => {
                    self.current.insert(key.clone(), Some(data.clone()));
                    if current != data {
                        Some(Overwrite { key, data })
                    } else {
                        None
                    }
                }
                // Existing entity was deleted
                (Some(_), EntityOp::Remove) => {
                    self.current.insert(key.clone(), None);
                    Some(Remove { key })
                }
                // Entity was deleted, but it doesn't exist in the store
                (None, EntityOp::Remove) => None,
            };
            if let Some(modification) = modification {
                mods.push(modification)
            }
        }
        Ok(ModificationsAndCache {
            modifications: mods,
            entity_lfu_cache: self.current,
        })
    }
}

impl Debug for EntityCache {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EntityCache")
            .field("current", &self.current)
            .field("updates", &self.updates)
            .finish()
    }
}

//
// impl EntityCache {
//
//     /// Add a dynamic data source
//     pub fn add_data_source<C: Blockchain>(&mut self, data_source: &impl DataSource<C>) {
//         self.data_sources
//             .push(data_source.as_stored_dynamic_data_source());
//     }
//
//
//
// }
//
// impl LfuCache<EntityKey, Option<Entity>> {
//     // Helper for cached lookup of an entity.
//     fn get_entity(
//         &mut self,
//         store: &(impl WritableStore + ?Sized),
//         key: &EntityKey,
//     ) -> Result<Option<Entity>, QueryExecutionError> {
//         match self.get(key) {
//             None => {
//                 let mut entity = store.get(key)?;
//                 if let Some(entity) = &mut entity {
//                     // `__typename` is for queries not for mappings.
//                     entity.remove("__typename");
//                 }
//                 self.insert(key.clone(), entity.clone());
//                 Ok(entity)
//             }
//             Some(data) => Ok(data.to_owned()),
//         }
//     }
// }
