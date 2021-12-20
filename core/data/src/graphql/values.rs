use crate::prelude::q;
use massbit_common::prelude::anyhow::{anyhow, Error};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

pub trait TryFromValue: Sized {
    fn try_from_value(value: &q::Value) -> Result<Self, Error>;
}

impl TryFromValue for q::Value {
    fn try_from_value(value: &q::Value) -> Result<Self, Error> {
        Ok(value.clone())
    }
}

impl TryFromValue for bool {
    fn try_from_value(value: &q::Value) -> Result<Self, Error> {
        match value {
            q::Value::Boolean(b) => Ok(*b),
            _ => Err(anyhow!("Cannot parse value into a boolean: {:?}", value)),
        }
    }
}

impl TryFromValue for String {
    fn try_from_value(value: &q::Value) -> Result<Self, Error> {
        match value {
            q::Value::String(s) => Ok(s.clone()),
            q::Value::Enum(s) => Ok(s.clone()),
            _ => Err(anyhow!("Cannot parse value into a string: {:?}", value)),
        }
    }
}

impl TryFromValue for u64 {
    fn try_from_value(value: &q::Value) -> Result<Self, Error> {
        match value {
            q::Value::Int(n) => n
                .as_i64()
                .map(|n| n as u64)
                .ok_or_else(|| anyhow!("Cannot parse value into an integer/u64: {:?}", n)),

            // `BigInt`s are represented as `String`s.
            q::Value::String(s) => u64::from_str(s).map_err(Into::into),
            _ => Err(anyhow!(
                "Cannot parse value into an integer/u64: {:?}",
                value
            )),
        }
    }
}
