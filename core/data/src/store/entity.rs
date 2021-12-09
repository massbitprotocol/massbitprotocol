use super::Value;
use crate::prelude::{q, s};
use crate::store::chain::BlockNumber;
use crate::store::deployment::{DeploymentHash, DeploymentLocator};
use crate::store::value::ValueType;
use crate::utils::cache_weight::CacheWeight;
use core::cmp::Eq;
use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::anyhow::{anyhow, Error};
use serde_derive::{Deserialize, Serialize};
use slog::Logger;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::iter::FromIterator;

/// An entity attribute name is represented as a string.
pub type Attribute = String;

/// The type name of an entity. This is the string that is used in the
/// indexer's GraphQL schema as `type NAME @entity { .. }`
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

impl From<&s::ObjectType> for EntityType {
    fn from(object_type: &s::ObjectType) -> Self {
        EntityType::new(object_type.name.to_owned())
    }
}

impl From<&s::InterfaceType> for EntityType {
    fn from(interface_type: &s::InterfaceType) -> Self {
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
impl CacheWeight for EntityType {
    fn indirect_weight(&self) -> usize {
        0
    }
}

// Note: Do not modify fields without making a backward compatible change to
// the StableHash impl (below)
/// Key by which an individual entity in the store can be accessed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityKey {
    /// ID of the subgraph.
    pub subgraph_id: DeploymentHash,

    /// Name of the entity type.
    pub entity_type: EntityType,

    /// ID of the individual entity.
    pub entity_id: String,
}
impl CacheWeight for EntityKey {
    fn indirect_weight(&self) -> usize {
        self.subgraph_id.indirect_weight()
            + self.entity_id.indirect_weight()
            + self.entity_type.indirect_weight()
    }
}
// Note: Do not modify fields without making a backward compatible change to the
//  StableHash impl (below) An entity is represented as a map of attribute names
//  to values.
/// An entity is represented as a map of attribute names to values.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Entity(HashMap<Attribute, Value>);

#[macro_export]
macro_rules! entity {
    ($($name:ident: $value:expr,)*) => {
        {
            let mut result = $crate::data::store::Entity::new();
            $(
                result.set(stringify!($name), $crate::data::store::Value::from($value));
            )*
            result
        }
    };
    ($($name:ident: $value:expr),*) => {
        entity! {$($name: $value,)*}
    };
}

impl Entity {
    /// Creates a new entity with no attributes set.
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        self.0.insert(key, value)
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.remove(key)
    }

    pub fn contains_key(&mut self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    // This collects the entity into an ordered vector so that it can be iterated deterministically.
    pub fn sorted(self) -> Vec<(String, Value)> {
        let mut v: Vec<_> = self.0.into_iter().collect();
        v.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        v
    }

    /// Try to get this entity's ID
    pub fn id(&self) -> Result<String, Error> {
        match self.get("id") {
            None => Err(anyhow!("Entity is missing an `id` attribute")),
            Some(Value::String(s)) => Ok(s.to_owned()),
            _ => Err(anyhow!("Entity has non-string `id` attribute")),
        }
    }

    /// Convenience method to save having to `.into()` the arguments.
    pub fn set(&mut self, name: impl Into<Attribute>, value: impl Into<Value>) -> Option<Value> {
        self.0.insert(name.into(), value.into())
    }

    /// Merges an entity update `update` into this entity.
    ///
    /// If a key exists in both entities, the value from `update` is chosen.
    /// If a key only exists on one entity, the value from that entity is chosen.
    /// If a key is set to `Value::Null` in `update`, the key/value pair is set to `Value::Null`.
    pub fn merge(&mut self, update: Entity) {
        for (key, value) in update.0.into_iter() {
            self.insert(key, value);
        }
    }

    /// Merges an entity update `update` into this entity, removing `Value::Null` values.
    ///
    /// If a key exists in both entities, the value from `update` is chosen.
    /// If a key only exists on one entity, the value from that entity is chosen.
    /// If a key is set to `Value::Null` in `update`, the key/value pair is removed.
    pub fn merge_remove_null_fields(&mut self, update: Entity) {
        for (key, value) in update.0.into_iter() {
            match value {
                Value::Null => self.remove(&key),
                _ => self.insert(key, value),
            };
        }
    }
}

impl From<Entity> for BTreeMap<String, q::Value> {
    fn from(entity: Entity) -> BTreeMap<String, q::Value> {
        entity.0.into_iter().map(|(k, v)| (k, v.into())).collect()
    }
}

impl From<Entity> for q::Value {
    fn from(entity: Entity) -> q::Value {
        q::Value::Object(entity.into())
    }
}

impl From<HashMap<Attribute, Value>> for Entity {
    fn from(m: HashMap<Attribute, Value>) -> Entity {
        Entity(m)
    }
}

impl<'a> From<Vec<(&'a str, Value)>> for Entity {
    fn from(entries: Vec<(&'a str, Value)>) -> Entity {
        Entity::from(HashMap::from_iter(
            entries.into_iter().map(|(k, v)| (String::from(k), v)),
        ))
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

/// A query for entities in a store.
///
/// Details of how query generation for `EntityQuery` works can be found
/// at https://github.com/graphprotocol/rfcs/blob/master/engineering-plans/0001-graphql-query-prefetching.md
#[derive(Clone, Debug)]
pub struct EntityQuery {
    /// ID of the subgraph.
    pub subgraph_id: DeploymentHash,

    /// The block height at which to execute the query. Set this to
    /// `BLOCK_NUMBER_MAX` to run the query at the latest available block.
    /// If the subgraph uses JSONB storage, anything but `BLOCK_NUMBER_MAX`
    /// will cause an error as JSONB storage does not support querying anything
    /// but the latest block
    pub block: BlockNumber,

    /// The names of the entity types being queried. The result is the union
    /// (with repetition) of the query for each entity.
    pub collection: EntityCollection,

    /// Filter to filter entities by.
    pub filter: Option<EntityFilter>,

    /// How to order the entities
    pub order: EntityOrder,

    /// A range to limit the size of the result.
    pub range: EntityRange,

    /// Optional logger for anything related to this query
    pub logger: Option<Logger>,

    pub query_id: Option<String>,

    _force_use_of_new: (),
}

impl EntityQuery {
    pub fn new(
        subgraph_id: DeploymentHash,
        block: BlockNumber,
        collection: EntityCollection,
    ) -> Self {
        EntityQuery {
            subgraph_id,
            block,
            collection,
            filter: None,
            order: EntityOrder::Default,
            range: EntityRange::first(100),
            logger: None,
            query_id: None,
            _force_use_of_new: (),
        }
    }

    pub fn filter(mut self, filter: EntityFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn order(mut self, order: EntityOrder) -> Self {
        self.order = order;
        self
    }

    pub fn range(mut self, range: EntityRange) -> Self {
        self.range = range;
        self
    }

    pub fn first(mut self, first: u32) -> Self {
        self.range.first = Some(first);
        self
    }

    pub fn skip(mut self, skip: u32) -> Self {
        self.range.skip = skip;
        self
    }

    pub fn simplify(mut self) -> Self {
        // If there is one window, with one id, in a direct relation to the
        // entities, we can simplify the query by changing the filter and
        // getting rid of the window
        if let EntityCollection::Window(windows) = &self.collection {
            if windows.len() == 1 {
                let window = windows.first().expect("we just checked");
                if window.ids.len() == 1 {
                    let id = window.ids.first().expect("we just checked");
                    if let EntityLink::Direct(attribute, _) = &window.link {
                        let filter = match attribute {
                            WindowAttribute::Scalar(name) => {
                                EntityFilter::Equal(name.to_owned(), id.into())
                            }
                            WindowAttribute::List(name) => {
                                EntityFilter::Contains(name.to_owned(), Value::from(vec![id]))
                            }
                        };
                        self.filter = Some(filter.and_maybe(self.filter));
                        self.collection = EntityCollection::All(vec![(
                            window.child_type.to_owned(),
                            window.column_names.clone(),
                        )]);
                    }
                }
            }
        }
        self
    }
}

/// Operation types that lead to entity changes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum EntityChangeOperation {
    /// An entity was added or updated
    Set,
    /// An existing entity was removed.
    Removed,
}

/// Entity change events emitted by [Store](trait.Store.html) implementations.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityChange {
    Data {
        subgraph_id: DeploymentHash,
        /// Entity type name of the changed entity.
        entity_type: EntityType,
    },
    Assignment {
        deployment: DeploymentLocator,
        operation: EntityChangeOperation,
    },
}

impl EntityChange {
    pub fn for_data(key: EntityKey) -> Self {
        Self::Data {
            subgraph_id: key.subgraph_id,
            entity_type: key.entity_type,
        }
    }

    pub fn for_assignment(deployment: DeploymentLocator, operation: EntityChangeOperation) -> Self {
        Self::Assignment {
            deployment,
            operation,
        }
    }

    pub fn as_filter(&self) -> SubscriptionFilter {
        use EntityChange::*;
        match self {
            Data {
                subgraph_id,
                entity_type,
                ..
            } => SubscriptionFilter::Entities(subgraph_id.clone(), entity_type.clone()),
            Assignment { .. } => SubscriptionFilter::Assignment,
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

/// Filter subscriptions
pub enum SubscriptionFilter {
    /// Receive updates about all entities from the given deployment of the
    /// given type
    Entities(DeploymentHash, EntityType),
    /// Subscripe to changes in deployment assignments
    Assignment,
}

impl SubscriptionFilter {
    pub fn matches(&self, change: &EntityChange) -> bool {
        match (self, change) {
            (
                Self::Entities(eid, etype),
                EntityChange::Data {
                    subgraph_id,
                    entity_type,
                    ..
                },
            ) => subgraph_id == eid && entity_type == etype,
            (Self::Assignment, EntityChange::Assignment { .. }) => true,
            _ => false,
        }
    }
}

/// Supported types of store filters.
#[derive(Clone, Debug, PartialEq)]
pub enum EntityFilter {
    And(Vec<EntityFilter>),
    Or(Vec<EntityFilter>),
    Equal(Attribute, Value),
    Not(Attribute, Value),
    GreaterThan(Attribute, Value),
    LessThan(Attribute, Value),
    GreaterOrEqual(Attribute, Value),
    LessOrEqual(Attribute, Value),
    In(Attribute, Vec<Value>),
    NotIn(Attribute, Vec<Value>),
    Contains(Attribute, Value),
    NotContains(Attribute, Value),
    StartsWith(Attribute, Value),
    NotStartsWith(Attribute, Value),
    EndsWith(Attribute, Value),
    NotEndsWith(Attribute, Value),
}

// Define some convenience methods
impl EntityFilter {
    pub fn new_equal(
        attribute_name: impl Into<Attribute>,
        attribute_value: impl Into<Value>,
    ) -> Self {
        EntityFilter::Equal(attribute_name.into(), attribute_value.into())
    }

    pub fn new_in(
        attribute_name: impl Into<Attribute>,
        attribute_values: Vec<impl Into<Value>>,
    ) -> Self {
        EntityFilter::In(
            attribute_name.into(),
            attribute_values.into_iter().map(Into::into).collect(),
        )
    }

    pub fn and_maybe(self, other: Option<Self>) -> Self {
        use EntityFilter as f;
        match other {
            Some(other) => match (self, other) {
                (f::And(mut fs1), f::And(mut fs2)) => {
                    fs1.append(&mut fs2);
                    f::And(fs1)
                }
                (f::And(mut fs1), f2) => {
                    fs1.push(f2);
                    f::And(fs1)
                }
                (f1, f::And(mut fs2)) => {
                    fs2.push(f1);
                    f::And(fs2)
                }
                (f1, f2) => f::And(vec![f1, f2]),
            },
            None => self,
        }
    }
}

/// The order in which entities should be restored from a store.
#[derive(Clone, Debug, PartialEq)]
pub enum EntityOrder {
    /// Order ascending by the given attribute. Use `id` as a tie-breaker
    Ascending(String, ValueType),
    /// Order descending by the given attribute. Use `id` as a tie-breaker
    Descending(String, ValueType),
    /// Order by the `id` of the entities
    Default,
    /// Do not order at all. This speeds up queries where we know that
    /// order does not matter
    Unordered,
}

/// How many entities to return, how many to skip etc.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityRange {
    /// Limit on how many entities to return.
    pub first: Option<u32>,

    /// How many entities to skip.
    pub skip: u32,
}

impl EntityRange {
    /// Query for the first `n` entities.
    pub fn first(n: u32) -> Self {
        Self {
            first: Some(n),
            skip: 0,
        }
    }
}
/// The attribute we want to window by in an `EntityWindow`. We have to
/// distinguish between scalar and list attributes since we need to use
/// different queries for them, and the JSONB storage scheme can not
/// determine that by itself
#[derive(Clone, Debug, PartialEq)]
pub enum WindowAttribute {
    Scalar(String),
    List(String),
}

impl WindowAttribute {
    pub fn name(&self) -> &str {
        match self {
            WindowAttribute::Scalar(name) => name,
            WindowAttribute::List(name) => name,
        }
    }
}

/// How to connect children to their parent when the child table does not
/// store parent id's
#[derive(Clone, Debug, PartialEq)]
pub enum ParentLink {
    /// The parent stores a list of child ids. The ith entry in the outer
    /// vector contains the id of the children for `EntityWindow.ids[i]`
    List(Vec<Vec<String>>),
    /// The parent stores the id of one child. The ith entry in the
    /// vector contains the id of the child of the parent with id
    /// `EntityWindow.ids[i]`
    Scalar(Vec<String>),
}

/// How many children a parent can have when the child stores
/// the id of the parent
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChildMultiplicity {
    Single,
    Many,
}

/// How to select children for their parents depending on whether the
/// child stores parent ids (`Direct`) or the parent
/// stores child ids (`Parent`)
#[derive(Clone, Debug, PartialEq)]
pub enum EntityLink {
    /// The parent id is stored in this child attribute
    Direct(WindowAttribute, ChildMultiplicity),
    /// Join with the parents table to get at the parent id
    Parent(ParentLink),
}

/// Window results of an `EntityQuery` query along the parent's id:
/// the `order_by`, `order_direction`, and `range` of the query apply to
/// entities that belong to the same parent. Only entities that belong to
/// one of the parents listed in `ids` will be included in the query result.
///
/// Note that different windows can vary both by the entity type and id of
/// the children, but also by how to get from a child to its parent, i.e.,
/// it is possible that two windows access the same entity type, but look
/// at different attributes to connect to parent entities
#[derive(Clone, Debug, PartialEq)]
pub struct EntityWindow {
    /// The entity type for this window
    pub child_type: EntityType,
    /// The ids of parents that should be considered for this window
    pub ids: Vec<String>,
    /// How to get the parent id
    pub link: EntityLink,
    pub column_names: AttributeNames,
}

/// The base collections from which we are going to get entities for use in
/// `EntityQuery`; the result of the query comes from applying the query's
/// filter and order etc. to the entities described in this collection. For
/// a windowed collection order and range are applied to each individual
/// window
#[derive(Clone, Debug, PartialEq)]
pub enum EntityCollection {
    /// Use all entities of the given types
    All(Vec<(EntityType, AttributeNames)>),
    /// Use entities according to the windows. The set of entities that we
    /// apply order and range to is formed by taking all entities matching
    /// the window, and grouping them by the attribute of the window. Entities
    /// that have the same value in the `attribute` field of their window are
    /// grouped together. Note that it is possible to have one window for
    /// entity type `A` and attribute `a`, and another for entity type `B` and
    /// column `b`; they will be grouped by using `A.a` and `B.b` as the keys
    Window(Vec<EntityWindow>),
}

impl EntityCollection {
    pub fn entity_types_and_column_names(&self) -> BTreeMap<EntityType, AttributeNames> {
        let mut map = BTreeMap::new();
        match self {
            EntityCollection::All(pairs) => pairs.iter().for_each(|(entity_type, column_names)| {
                map.insert(entity_type.clone(), column_names.clone());
            }),
            EntityCollection::Window(windows) => windows.iter().for_each(
                |EntityWindow {
                     child_type,
                     column_names,
                     ..
                 }| match map.entry(child_type.clone()) {
                    Entry::Occupied(mut entry) => entry.get_mut().extend(column_names.clone()),
                    Entry::Vacant(entry) => {
                        entry.insert(column_names.clone());
                    }
                },
            ),
        }
        map
    }
}

/// Determines which columns should be selected in a table.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AttributeNames {
    /// Select all columns. Equivalent to a `"SELECT *"`.
    All,
    /// Individual column names to be selected.
    Select(BTreeSet<String>),
}

impl AttributeNames {
    pub fn insert(&mut self, column_name: &str) {
        match self {
            AttributeNames::All => {
                let mut set = BTreeSet::new();
                set.insert(column_name.to_string());
                *self = AttributeNames::Select(set)
            }
            AttributeNames::Select(set) => {
                set.insert(column_name.to_string());
            }
        }
    }

    pub fn update(&mut self, field: &q::Field) {
        // ignore "meta" field names
        if field.name.starts_with("__") {
            return;
        }
        self.insert(&field.name)
    }

    pub fn extend(&mut self, other: Self) {
        use AttributeNames::*;
        match (self, other) {
            (All, All) => {}
            (self_ @ All, other @ Select(_)) => *self_ = other,
            (Select(_), All) => {
                unreachable!()
            }
            (Select(a), Select(b)) => a.extend(b),
        }
    }
}
