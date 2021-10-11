use crate::data::indexer::DeploymentHash;
use crate::prelude::{q, CacheWeight, EntityKey};
use anyhow::{anyhow, Error};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use stable_hash::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;
use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;
use strum::AsStaticRef as _;
use strum_macros::AsStaticStr;

pub mod ethereum;
/// Custom scalars in GraphQL.
pub mod scalar;
/// An entity attribute name is represented as a string.
pub type Attribute = String;

pub const ID: &str = "ID";
pub const BYTES_SCALAR: &str = "Bytes";
pub const BIG_INT_SCALAR: &str = "BigInt";
pub const BIG_DECIMAL_SCALAR: &str = "BigDecimal";

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
                Value::List(ref values) =>
                    format!("[{}]", values.iter().map(ToString::to_string).join(", ")),
                Value::Bytes(ref bytes) => bytes.to_string(),
                Value::BigInt(ref number) => number.to_string(),
            }
        )
    }
}

impl From<Value> for q::Value {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => q::Value::String(s),
            Value::Int(i) => q::Value::Int(q::Number::from(i)),
            Value::BigDecimal(d) => q::Value::String(d.to_string()),
            Value::Bool(b) => q::Value::Boolean(b),
            Value::Null => q::Value::Null,
            Value::List(values) => {
                q::Value::List(values.into_iter().map(|value| value.into()).collect())
            }
            Value::Bytes(bytes) => q::Value::String(bytes.to_string()),
            Value::BigInt(number) => q::Value::String(number.to_string()),
        }
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
    fn to_entity_key(&self, subgraph: DeploymentHash) -> EntityKey;
}
