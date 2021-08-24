use crate::graph::cheap_clone::CheapClone;
use crate::stable_hash::{SequenceNumber, StableHash, StableHasher};
use crate::store::{scalar, StoreError};
use crate::util::cache_weight::CacheWeight;
use massbit_common::prelude::anyhow::{anyhow, Error};
use massbit_common::prelude::diesel;
use massbit_common::prelude::diesel::deserialize::FromSql;
use massbit_common::prelude::diesel::pg::Pg;
use massbit_common::prelude::diesel::serialize::{Output, ToSql};
use massbit_common::prelude::diesel::sql_types::Text;
use massbit_common::prelude::lazy_static::lazy_static;
use massbit_common::prelude::{
    diesel_derives::{AsExpression, FromSqlRow},
    //serde::__private::fmt::Write,
    serde_derive::{Deserialize, Serialize},
};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::io::Write;
use std::iter::FromIterator;
use std::str::FromStr;
use std::time::Duration;
use strum::AsStaticRef as _;
use strum_macros::AsStaticStr;

/// The type we use for block numbers. This has to be a signed integer type
/// since Postgres does not support unsigned integer types. But 2G ought to
/// be enough for everybody
pub type BlockNumber = i32;
/// An entity attribute name is represented as a string.
pub type Attribute = String;

pub const ID: &str = "ID";
pub const BYTES_SCALAR: &str = "Bytes";
pub const BIG_INT_SCALAR: &str = "BigInt";
pub const BIG_DECIMAL_SCALAR: &str = "BigDecimal";

/// The name of a database shard; valid names must match `[a-z0-9_]+`
#[derive(Clone, Debug, Eq, PartialEq, Hash, AsExpression, FromSqlRow)]
pub struct ShardName(String);

lazy_static! {
    /// The name of the primary shard that contains all instance-wide data
    pub static ref PRIMARY_SHARD: ShardName = ShardName("primary".to_string());
}

/// How long to cache information about a deployment site
const SITES_CACHE_TTL: Duration = Duration::from_secs(120);

impl ShardName {
    pub fn new(name: String) -> Result<Self, StoreError> {
        if name.is_empty() {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names must not be empty"
            )));
        }
        if name.len() > 30 {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names can be at most 30 characters, but `{}` has {} characters",
                name,
                name.len()
            )));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(StoreError::InvalidIdentifier(format!(
                "shard names must only contain lowercase alphanumeric characters or '_'"
            )));
        }
        Ok(ShardName(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ShardName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromSql<Text, Pg> for ShardName {
    fn from_sql(bytes: Option<&[u8]>) -> diesel::deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        ShardName::new(s).map_err(Into::into)
    }
}

impl ToSql<Text, Pg> for ShardName {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> diesel::serialize::Result {
        <String as ToSql<Text, Pg>>::to_sql(&self.0, out)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueType {
    Boolean,
    BigInt,
    Bytes,
    BigDecimal,
    Int,
    String,
}

impl FromStr for ValueType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Boolean" => Ok(ValueType::Boolean),
            "BigInt" => Ok(ValueType::BigInt),
            "Bytes" => Ok(ValueType::Bytes),
            "BigDecimal" => Ok(ValueType::BigDecimal),
            "Int" => Ok(ValueType::Int),
            "String" | "ID" => Ok(ValueType::String),
            s => Err(anyhow!("Type not available in this context: {}", s)),
        }
    }
}

impl ValueType {
    /// Return `true` if `s` is the name of a builtin scalar type
    pub fn is_scalar(s: &str) -> bool {
        Self::from_str(s).is_ok()
    }
}

// Note: Do not modify fields without also making a backward compatible change to the StableHash impl (below)
/// An attribute value is represented as an enum with variants for all supported value types.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
#[derive(AsStaticStr)]
pub enum Value {
    String(String),
    Int(i32),
    BigDecimal(scalar::BigDecimal),
    Bool(bool),
    List(Vec<Value>),
    Null,
    Bytes(scalar::Bytes),
    BigInt(scalar::BigInt),
}

impl StableHash for Value {
    fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
        use Value::*;

        // This is the default, so write nothing.
        match self {
            Null => return,
            _ => {}
        }

        self.as_static()
            .stable_hash(sequence_number.next_child(), state);

        match self {
            Null => unreachable!(),
            String(inner) => inner.stable_hash(sequence_number, state),
            Int(inner) => inner.stable_hash(sequence_number, state),
            BigDecimal(inner) => inner.stable_hash(sequence_number, state),
            Bool(inner) => inner.stable_hash(sequence_number, state),
            List(inner) => inner.stable_hash(sequence_number, state),
            Bytes(inner) => inner.stable_hash(sequence_number, state),
            BigInt(inner) => inner.stable_hash(sequence_number, state),
        }
    }
}

impl Value {
    /*
    pub fn from_query_value(value: &q::Value, ty: &s::Type) -> Result<Value, QueryExecutionError> {
        use graphql_parser::schema::Type::{ListType, NamedType, NonNullType};

        Ok(match (value, ty) {
            // When dealing with non-null types, use the inner type to convert the value
            (value, NonNullType(t)) => Value::from_query_value(value, t)?,

            (q::Value::List(values), ListType(ty)) => Value::List(
                values
                    .iter()
                    .map(|value| Self::from_query_value(value, ty))
                    .collect::<Result<Vec<_>, _>>()?,
            ),

            (q::Value::List(values), NamedType(n)) => Value::List(
                values
                    .iter()
                    .map(|value| Self::from_query_value(value, &NamedType(n.to_string())))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            (q::Value::Enum(e), NamedType(_)) => Value::String(e.clone()),
            (q::Value::String(s), NamedType(n)) => {
                // Check if `ty` is a custom scalar type, otherwise assume it's
                // just a string.
                match n.as_str() {
                    BYTES_SCALAR => Value::Bytes(scalar::Bytes::from_str(s)?),
                    BIG_INT_SCALAR => Value::BigInt(scalar::BigInt::from_str(s)?),
                    BIG_DECIMAL_SCALAR => Value::BigDecimal(scalar::BigDecimal::from_str(s)?),
                    _ => Value::String(s.clone()),
                }
            }
            (q::Value::Int(i), _) => Value::Int(
                i.to_owned()
                    .as_i64()
                    .ok_or_else(|| QueryExecutionError::NamedTypeError("Int".to_string()))?
                    as i32,
            ),
            (q::Value::Boolean(b), _) => Value::Bool(b.to_owned()),
            (q::Value::Null, _) => Value::Null,
            _ => {
                return Err(QueryExecutionError::AttributeTypeError(
                    value.to_string(),
                    ty.to_string(),
                ));
            }
        })
    }
    */
    pub fn as_string(self) -> Option<String> {
        if let Value::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(s) = self {
            Some(s.as_str())
        } else {
            None
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    pub fn as_int(self) -> Option<i32> {
        if let Value::Int(i) = self {
            Some(i)
        } else {
            None
        }
    }

    pub fn as_big_decimal(self) -> Option<scalar::BigDecimal> {
        if let Value::BigDecimal(d) = self {
            Some(d)
        } else {
            None
        }
    }

    pub fn as_bool(self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_list(self) -> Option<Vec<Value>> {
        if let Value::List(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_bytes(self) -> Option<scalar::Bytes> {
        if let Value::Bytes(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_bigint(self) -> Option<scalar::BigInt> {
        if let Value::BigInt(b) = self {
            Some(b)
        } else {
            None
        }
    }

    /// Return the name of the type of this value for display to the user
    pub fn type_name(&self) -> String {
        match self {
            Value::BigDecimal(_) => "BigDecimal".to_owned(),
            Value::BigInt(_) => "BigInt".to_owned(),
            Value::Bool(_) => "Boolean".to_owned(),
            Value::Bytes(_) => "Bytes".to_owned(),
            Value::Int(_) => "Int".to_owned(),
            Value::List(values) => {
                if let Some(v) = values.first() {
                    format!("[{}]", v.type_name())
                } else {
                    "[Any]".to_owned()
                }
            }
            Value::Null => "Null".to_owned(),
            Value::String(_) => "String".to_owned(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Value::String(s) => s.to_string(),
                Value::Int(i) => i.to_string(),
                Value::BigDecimal(d) => d.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                Value::List(ref values) => format!(
                    "[{}]",
                    values
                        .into_iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Value::Bytes(ref bytes) => bytes.to_string(),
                Value::BigInt(ref number) => number.to_string(),
            }
        )
    }
}
impl<'a> From<&'a str> for Value {
    fn from(value: &'a str) -> Value {
        Value::String(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Value {
        Value::String(value)
    }
}

impl<'a> From<&'a String> for Value {
    fn from(value: &'a String) -> Value {
        Value::String(value.clone())
    }
}

impl From<scalar::Bytes> for Value {
    fn from(value: scalar::Bytes) -> Value {
        Value::Bytes(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Value {
        Value::Bool(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Value {
        Value::Int(value)
    }
}

impl From<scalar::BigDecimal> for Value {
    fn from(value: scalar::BigDecimal) -> Value {
        Value::BigDecimal(value)
    }
}

impl From<scalar::BigInt> for Value {
    fn from(value: scalar::BigInt) -> Value {
        Value::BigInt(value)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Value {
        Value::BigInt(value.into())
    }
}

impl TryFrom<Value> for Option<scalar::BigInt> {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::BigInt(n) => Ok(Some(n)),
            Value::Null => Ok(None),
            _ => Err(anyhow!("Value is not an BigInt")),
        }
    }
}

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(values: Vec<T>) -> Value {
        Value::List(values.into_iter().map(Into::into).collect())
    }
}

impl<T> From<Option<T>> for Value
where
    Value: From<T>,
{
    fn from(x: Option<T>) -> Value {
        match x {
            Some(x) => x.into(),
            None => Value::Null,
        }
    }
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
/*
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
*/
// This conversion should only be used in tests since it makes it too
// easy to convert random strings into entity types
#[cfg(debug_assertions)]
impl From<&str> for EntityType {
    fn from(s: &str) -> Self {
        EntityType::new(s.to_owned())
    }
}

impl CheapClone for EntityType {}

// Note: Do not modify fields without making a backward compatible change to the
//  StableHash impl (below) An entity is represented as a map of attribute names
//  to values.
/// An entity is represented as a map of attribute names to values.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Entity(HashMap<Attribute, Value>);

impl StableHash for Entity {
    #[inline]
    fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
        self.0.stable_hash(sequence_number.next_child(), state);
    }
}

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

impl CacheWeight for Entity {
    fn indirect_weight(&self) -> usize {
        self.0.indirect_weight()
    }
}

/// A value that can (maybe) be converted to an `Entity`.
pub trait TryIntoEntity {
    fn try_into_entity(self) -> Result<Entity, Error>;
}

/// A value that can be converted to an `Entity` ID.
pub trait ToEntityId {
    fn to_entity_id(&self) -> String;
}

/// A value that can be converted to an `Entity` key.
pub trait ToEntityKey {
    fn to_entity_key(&self, indexer: String) -> EntityKey;
}

// Note: Do not modify fields without making a backward compatible change to
// the StableHash impl (below)
/// Key by which an individual entity in the store can be accessed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityKey {
    /// ID of the subgraph.
    pub indexer_id: String,

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
    pub fn data(indexer_id: String, entity_type: String, entity_id: String) -> Self {
        Self {
            indexer_id,
            entity_type: EntityType::new(entity_type),
            entity_id,
        }
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
pub enum EntityOp {
    Remove,
    Update(Entity),
    Overwrite(Entity),
}

impl EntityOp {
    pub fn apply_to(self, entity: Option<Entity>) -> Option<Entity> {
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

    pub fn accumulate(&mut self, next: EntityOp) {
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
