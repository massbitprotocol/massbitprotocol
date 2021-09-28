use futures::stream::poll_fn;
use futures::{Async, Poll, Stream};
use graphql_parser::schema as s;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use stable_hash::prelude::*;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use thiserror::Error;
use web3::types::{Address, H256};

use crate::blockchain::{Blockchain, DataSource};
use crate::data::indexer::Source;
use crate::data::query::QueryExecutionError;
use crate::data::store::Entity;
use crate::prelude::*;
use crate::util::lfu_cache::LfuCache;

/// The type we use for block numbers. This has to be a signed integer type
/// since Postgres does not support unsigned integer types. But 2G ought to
/// be enough for everybody
pub type BlockNumber = i32;

pub const BLOCK_NUMBER_MAX: BlockNumber = std::i32::MAX;

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
    current: LfuCache<EntityKey, Option<Entity>>,

    /// The accumulated changes to an entity.
    updates: HashMap<EntityKey, EntityOp>,

    // Updates for a currently executing handler.
    handler_updates: HashMap<EntityKey, EntityOp>,

    // Marks whether updates should go in `handler_updates`.
    in_handler: bool,

    data_sources: Vec<StoredDynamicDataSource>,

    /// The store is only used to read entities.
    pub store: Arc<dyn WritableStore>,
}

impl Debug for EntityCache {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EntityCache")
            .field("current", &self.current)
            .field("updates", &self.updates)
            .finish()
    }
}

pub struct ModificationsAndCache {
    pub modifications: Vec<EntityModification>,
    pub data_sources: Vec<StoredDynamicDataSource>,
    pub entity_lfu_cache: LfuCache<EntityKey, Option<Entity>>,
}

impl EntityCache {
    pub fn new(store: Arc<dyn WritableStore>) -> Self {
        Self {
            current: LfuCache::new(),
            updates: HashMap::new(),
            handler_updates: HashMap::new(),
            in_handler: false,
            data_sources: vec![],
            store,
        }
    }

    pub fn with_current(
        store: Arc<dyn WritableStore>,
        current: LfuCache<EntityKey, Option<Entity>>,
    ) -> EntityCache {
        EntityCache {
            current,
            updates: HashMap::new(),
            handler_updates: HashMap::new(),
            in_handler: false,
            data_sources: vec![],
            store,
        }
    }

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

    pub fn get(&mut self, key: &EntityKey) -> Result<Option<Entity>, QueryExecutionError> {
        // Get the current entity, apply any updates from `updates`, then from `handler_updates`.
        let mut entity = self.current.get_entity(&*self.store, key)?;
        if let Some(op) = self.updates.get(key).cloned() {
            entity = op.apply_to(entity)
        }
        if let Some(op) = self.handler_updates.get(key).cloned() {
            entity = op.apply_to(entity)
        }
        Ok(entity)
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

    /// Add a dynamic data source
    pub fn add_data_source<C: Blockchain>(&mut self, data_source: &impl DataSource<C>) {
        self.data_sources
            .push(data_source.as_stored_dynamic_data_source());
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

    pub(crate) fn extend(&mut self, other: EntityCache) {
        assert!(!other.in_handler);

        self.current.extend(other.current);
        for (key, op) in other.updates {
            self.entity_op(key, op);
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
                .entry(&key.indexer_id)
                .or_default()
                .entry(&key.entity_type)
                .or_default()
                .push(&key.entity_id);
        }

        for (indexer_id, keys) in missing_by_indexer {
            for (entity_type, entities) in self.store.get_many(keys)? {
                for entity in entities {
                    let key = EntityKey {
                        indexer_id: indexer_id.clone(),
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
            data_sources: self.data_sources,
            entity_lfu_cache: self.current,
        })
    }
}

impl LfuCache<EntityKey, Option<Entity>> {
    // Helper for cached lookup of an entity.
    fn get_entity(
        &mut self,
        store: &(impl WritableStore + ?Sized),
        key: &EntityKey,
    ) -> Result<Option<Entity>, QueryExecutionError> {
        match self.get(key) {
            None => {
                let mut entity = store.get(key)?;
                if let Some(entity) = &mut entity {
                    // `__typename` is for queries not for mappings.
                    entity.remove("__typename");
                }
                self.insert(key.clone(), entity.clone());
                Ok(entity)
            }
            Some(data) => Ok(data.to_owned()),
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

/// The type name of an entity. This is the string that is used in the
/// subgraph's GraphQL schema as `type NAME @entity { .. }`
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityType(String);

impl EntityType {
    /// Construct a new entity type. Ideally, this is only called when
    /// `entity_type` either comes from the GraphQL schema, or from
    /// the database from fields that are known to contain a valid entity type
    pub fn new(entity_type: String) -> Self {
        Self(entity_type)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> From<&s::ObjectType<'a, String>> for EntityType {
    fn from(object_type: &s::ObjectType<'a, String>) -> Self {
        EntityType::new(object_type.name.to_owned())
    }
}

impl<'a> From<&s::InterfaceType<'a, String>> for EntityType {
    fn from(interface_type: &s::InterfaceType<'a, String>) -> Self {
        EntityType::new(interface_type.name.to_owned())
    }
}

// This conversion should only be used in tests since it makes it too
// easy to convert random strings into entity types
#[cfg(debug_assertions)]
impl From<&str> for EntityType {
    fn from(s: &str) -> Self {
        EntityType::new(s.to_owned())
    }
}

impl CheapClone for EntityType {}

// Note: Do not modify fields without making a backward compatible change to
// the StableHash impl (below)
/// Key by which an individual entity in the store can be accessed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityKey {
    /// ID of the indexer.
    pub indexer_id: DeploymentHash,

    /// Name of the entity type.
    pub entity_type: EntityType,

    /// ID of the individual entity.
    pub entity_id: String,
}

impl StableHash for EntityKey {
    fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
        self.indexer_id
            .stable_hash(sequence_number.next_child(), state);
        self.entity_type
            .as_str()
            .stable_hash(sequence_number.next_child(), state);
        self.entity_id
            .stable_hash(sequence_number.next_child(), state);
    }
}

impl EntityKey {
    pub fn data(indexer_id: DeploymentHash, entity_type: String, entity_id: String) -> Self {
        Self {
            indexer_id,
            entity_type: EntityType::new(entity_type),
            entity_id,
        }
    }
}

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("store error: {0}")]
    Unknown(Error),
    #[error(
        "tried to set entity of type `{0}` with ID \"{1}\" but an entity of type `{2}`, \
         which has an interface in common with `{0}`, exists with the same ID"
    )]
    ConflictingId(String, String, String), // (entity, id, conflicting_entity)
    #[error("unknown field '{0}'")]
    UnknownField(String),
    #[error("unknown table '{0}'")]
    UnknownTable(String),
    #[error("malformed directive '{0}'")]
    MalformedDirective(String),
    #[error("query execution failed: {0}")]
    QueryExecutionError(String),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error(
        "subgraph `{0}` has already processed block `{1}`; \
         there are most likely two (or more) nodes indexing this subgraph"
    )]
    DuplicateBlockProcessing(DeploymentHash, BlockNumber),
    /// An internal error where we expected the application logic to enforce
    /// some constraint, e.g., that subgraph names are unique, but found that
    /// constraint to not hold
    #[error("internal constraint violated: {0}")]
    ConstraintViolation(String),
    #[error("deployment not found: {0}")]
    DeploymentNotFound(String),
    #[error("shard not found: {0} (this usually indicates a misconfiguration)")]
    UnknownShard(String),
    #[error("Fulltext search not yet deterministic")]
    FulltextSearchNonDeterministic,
    #[error("operation was canceled")]
    Canceled,
    #[error("database unavailable")]
    DatabaseUnavailable,
}

// Convenience to report a constraint violation
#[macro_export]
macro_rules! constraint_violation {
    ($msg:expr) => {{
        StoreError::ConstraintViolation(format!("{}", $msg))
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        StoreError::ConstraintViolation(format!($fmt, $($arg)*))
    }}
}

impl From<::diesel::result::Error> for StoreError {
    fn from(e: ::diesel::result::Error) -> Self {
        StoreError::Unknown(e.into())
    }
}

impl From<::diesel::r2d2::PoolError> for StoreError {
    fn from(e: ::diesel::r2d2::PoolError) -> Self {
        StoreError::Unknown(e.into())
    }
}

impl From<Error> for StoreError {
    fn from(e: Error) -> Self {
        StoreError::Unknown(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Unknown(e.into())
    }
}

impl From<QueryExecutionError> for StoreError {
    fn from(e: QueryExecutionError) -> Self {
        StoreError::QueryExecutionError(e.to_string())
    }
}

impl From<std::fmt::Error> for StoreError {
    fn from(e: std::fmt::Error) -> Self {
        StoreError::Unknown(anyhow!("{}", e.to_string()))
    }
}

pub struct StoredDynamicDataSource {
    pub name: String,
    pub source: Source,
    pub context: Option<String>,
    pub creation_block: Option<BlockNumber>,
}

#[async_trait]
pub trait WritableStore: Send + Sync + 'static {
    /// Looks up an entity using the given store key at the latest block.
    fn get(&self, key: &EntityKey) -> Result<Option<Entity>, QueryExecutionError>;

    /// Look up multiple entities as of the latest block. Returns a map of
    /// entities by type.
    fn get_many(
        &self,
        ids_for_type: BTreeMap<&EntityType, Vec<&str>>,
    ) -> Result<BTreeMap<EntityType, Vec<Entity>>, StoreError>;

    /// Transact the entity changes from a single block atomically into the store, and update the
    /// subgraph block pointer to `block_ptr_to`.
    ///
    /// `block_ptr_to` must point to a child block of the current subgraph block pointer.
    fn transact_block_operations(
        &self,
        block_ptr_to: BlockPtr,
        mods: Vec<EntityModification>,
        data_sources: Vec<StoredDynamicDataSource>,
    ) -> Result<(), StoreError>;

    /// Load the dynamic data sources for the given deployment
    async fn load_dynamic_data_sources(&self) -> Result<Vec<StoredDynamicDataSource>, StoreError>;
}

/// Common trait for store implementations.
#[async_trait]
pub trait IndexerStore: Send + Sync + 'static {
    /// Return a `WritableStore` that is used for indexer. Only
    /// code that is part of indexing a indexer should ever use this.
    fn writable(
        &self,
        deployment: &DeploymentLocator,
    ) -> Result<Arc<dyn WritableStore>, StoreError>;

    /// Create a new indexer with the given name. If one already exists, use
    /// the existing one. Return the `id` of the newly created or existing
    /// indexer
    fn create_indexer(&self, name: IndexerName) -> Result<String, StoreError>;

    /// Return the GraphQL schema supplied by the user
    fn input_schema(&self, indexer_id: &DeploymentHash) -> Result<Arc<Schema>, StoreError>;
}
