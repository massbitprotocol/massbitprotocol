use crate::prelude::{q, s, QueryExecutionError};
use crate::store::scalar;
use crate::utils::cache_weight::CacheWeight;
use massbit_common::prelude::anyhow::{anyhow, Error};
use serde_derive::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;
use strum::AsStaticRef as _;
use strum_macros::AsStaticStr;

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

impl Value {
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
impl From<i32> for Value {
    fn from(value: i32) -> Value {
        Value::Int(value)
    }
}
impl From<u64> for Value {
    fn from(value: u64) -> Value {
        Value::BigInt(value.into())
    }
}
impl From<u8> for Value {
    fn from(value: u8) -> Value {
        Value::Int(value as i32)
    }
}
impl From<i8> for Value {
    fn from(value: i8) -> Value {
        Value::Int(value as i32)
    }
}
impl From<u16> for Value {
    fn from(value: u16) -> Value {
        Value::Int(value as i32)
    }
}
impl From<i16> for Value {
    fn from(value: i16) -> Value {
        Value::Int(value as i32)
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

impl CacheWeight for Value {
    fn indirect_weight(&self) -> usize {
        match self {
            Value::String(s) => s.indirect_weight(),
            Value::BigDecimal(d) => d.indirect_weight(),
            Value::List(values) => values.indirect_weight(),
            Value::Bytes(bytes) => bytes.indirect_weight(),
            Value::BigInt(n) => n.indirect_weight(),
            Value::Int(_) | Value::Bool(_) | Value::Null => 0,
        }
    }
}
