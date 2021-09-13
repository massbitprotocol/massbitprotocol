use graph::data::store::{scalar, Entity, Value};
use std::any::Any;
use std::collections::{BTreeMap, HashMap};

pub trait FromEntity: Default {
    /// Converts a `GenericMap` back into a structure.
    /// __Constraints__: assumes that value types conform to the original types of the struct.
    fn from_entity(entity: &Entity) -> Self;
}

pub trait ToMap: Default {
    /// Generates a `GenericMap` where value types are all encapsulated under a sum type.
    /// __Constraints__: currently only supports primitive types for genericized values.
    #[allow(clippy::wrong_self_convention)]
    fn to_map(structure: Self) -> HashMap<String, Value>;
}
pub struct EntityValue {}

pub trait ValueFrom<T>: Sized {
    fn value_from(_: T) -> Value;
}
impl<'a> ValueFrom<&'a str> for EntityValue {
    fn value_from(value: &'a str) -> Value {
        Value::String(value.to_owned())
    }
}

impl ValueFrom<String> for EntityValue {
    fn value_from(value: String) -> Value {
        Value::String(value)
    }
}

impl<'a> ValueFrom<&'a String> for EntityValue {
    fn value_from(value: &'a String) -> Value {
        Value::String(value.clone())
    }
}

impl ValueFrom<scalar::Bytes> for EntityValue {
    fn value_from(value: scalar::Bytes) -> Value {
        Value::Bytes(value)
    }
}

impl ValueFrom<bool> for EntityValue {
    fn value_from(value: bool) -> Value {
        Value::Bool(value)
    }
}

impl ValueFrom<i32> for EntityValue {
    fn value_from(value: i32) -> Value {
        Value::Int(value)
    }
}

impl ValueFrom<scalar::BigDecimal> for EntityValue {
    fn value_from(value: scalar::BigDecimal) -> Value {
        Value::BigDecimal(value)
    }
}

impl ValueFrom<scalar::BigInt> for EntityValue {
    fn value_from(value: scalar::BigInt) -> Value {
        Value::BigInt(value)
    }
}

impl ValueFrom<u64> for EntityValue {
    fn value_from(value: u64) -> Value {
        Value::BigInt(value.into())
    }
}

impl ValueFrom<i64> for EntityValue {
    fn value_from(value: i64) -> Value {
        let val = value as u64;
        Value::BigInt(val.into())
    }
}

pub trait TryFrom {
    fn try_from<T: Any>(value: T) -> Value;
}
impl TryFrom for Value {
    fn try_from<T: Any>(value: T) -> Value {
        let any_val = &value as &dyn Any;
        if let Some(val) = any_val.downcast_ref::<bool>() {
            Value::Bool(*val)
        } else if let Some(val) = any_val.downcast_ref::<i64>() {
            Value::BigInt(scalar::BigInt::from(*val))
        } else if let Some(val) = any_val.downcast_ref::<u64>() {
            Value::BigInt(scalar::BigInt::from(*val))
        } else if let Some(val) = any_val.downcast_ref::<f64>() {
            Value::BigDecimal(scalar::BigDecimal::from(*val))
        } else if let Some(val) = any_val.downcast_ref::<i32>() {
            Value::Int(*val)
        } else if let Some(val) = any_val.downcast_ref::<u32>() {
            Value::Int(*val as i32)
        } else if let Some(val) = any_val.downcast_ref::<f32>() {
            Value::BigDecimal(scalar::BigDecimal::from(*val as f64))
        } else if let Some(val) = any_val.downcast_ref::<Vec<u8>>() {
            Value::Bytes(scalar::Bytes::from(val.as_slice()))
        } else if let Some(val) = any_val.downcast_ref::<&'static str>() {
            Value::String(val.to_string())
        } else if let Some(val) = any_val.downcast_ref::<String>() {
            Value::String(val.to_string())
        } else if let Some(val) = any_val.downcast_ref::<Vec<Value>>() {
            Value::List(val.to_vec())
        } else {
            Value::Null
        }
    }
}
pub trait FromValueTrait {
    fn as_i64(self) -> Option<i64>;
}
impl FromValueTrait for Value {
    fn as_i64(self) -> Option<i64> {
        if let Value::BigInt(big_int) = self {
            let val: u64 = big_int.to_u64();
            Some(val as i64)
        } else {
            None
        }
    }
}
