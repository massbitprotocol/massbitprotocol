use diesel::result::{Error as DieselError, QueryResult};
use massbit::components::store::StoreError;
use massbit::prelude::anyhow;
use massbit_common::prelude::serde_json;
use massbit_solana_sdk::entity::{Entity, Value};
use massbit_solana_sdk::scalar;
use massbit_store_postgres::relational::ColumnType;
use std::convert::TryFrom;
use std::str::FromStr;

pub trait FromColumnValue: Sized {
    fn is_null(&self) -> bool;

    fn null() -> Self;

    fn from_string(s: String) -> Self;

    fn from_bool(b: bool) -> Self;

    fn from_i32(i: i32) -> Self;

    fn from_big_decimal(d: scalar::BigDecimal) -> Self;

    fn from_big_int(i: serde_json::Number) -> Result<Self, StoreError>;

    // The string returned by the DB, without the leading '\x'
    fn from_bytes(i: &str) -> Result<Self, StoreError>;

    fn from_vec(v: Vec<Self>) -> Self;

    fn from_column_value(
        column_type: &ColumnType,
        json: serde_json::Value,
    ) -> Result<Self, StoreError> {
        use serde_json::Value as j;
        // Many possible conversion errors are already caught by how
        // we define the schema; for example, we can only get a NULL for
        // a column that is actually nullable
        match (json, column_type) {
            (j::Null, _) => Ok(Self::null()),
            (j::Bool(b), _) => Ok(Self::from_bool(b)),
            (j::Number(number), ColumnType::Int) => match number.as_i64() {
                Some(i) => i32::try_from(i).map(Self::from_i32).map_err(|e| {
                    StoreError::Unknown(anyhow!("failed to convert {} to Int: {}", number, e))
                }),
                None => Err(StoreError::Unknown(anyhow!(
                    "failed to convert {} to Int",
                    number
                ))),
            },
            (j::Number(number), ColumnType::BigDecimal) => {
                let s = number.to_string();
                scalar::BigDecimal::from_str(s.as_str())
                    .map(Self::from_big_decimal)
                    .map_err(|e| {
                        StoreError::Unknown(anyhow!(
                            "failed to convert {} to BigDecimal: {}",
                            number,
                            e
                        ))
                    })
            }
            (j::Number(number), ColumnType::BigInt) => Self::from_big_int(number),
            (j::Number(number), column_type) => Err(StoreError::Unknown(anyhow!(
                "can not convert number {} to {:?}",
                number,
                column_type
            ))),
            (j::String(s), ColumnType::String) | (j::String(s), ColumnType::Enum(_)) => {
                Ok(Self::from_string(s))
            }
            (j::String(s), ColumnType::Bytes) => Self::from_bytes(s.trim_start_matches("\\x")),
            (j::String(s), ColumnType::BytesId) => Ok(Self::from_string(bytes_as_str(&s))),
            (j::String(s), column_type) => Err(StoreError::Unknown(anyhow!(
                "can not convert string {} to {:?}",
                s,
                column_type
            ))),
            (j::Array(values), _) => Ok(Self::from_vec(
                values
                    .into_iter()
                    .map(|v| Self::from_column_value(column_type, v))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            (j::Object(_), _) => {
                unimplemented!("objects as entity attributes are not needed/supported")
            }
        }
    }
}

pub trait FromEntityData: Default + From<Entity> {
    type Value: FromColumnValue;

    fn insert_entity_data(&mut self, key: String, v: Self::Value);
}

impl FromColumnValue for Value {
    fn is_null(&self) -> bool {
        self == &Value::Null
    }

    fn null() -> Self {
        Self::Null
    }

    fn from_string(s: String) -> Self {
        Value::String(s)
    }

    fn from_bool(b: bool) -> Self {
        Value::Bool(b)
    }

    fn from_i32(i: i32) -> Self {
        Value::Int(i)
    }

    fn from_big_decimal(d: scalar::BigDecimal) -> Self {
        Value::BigDecimal(d)
    }

    fn from_big_int(i: serde_json::Number) -> Result<Self, StoreError> {
        scalar::BigInt::from_str(&i.to_string())
            .map(Value::BigInt)
            .map_err(|e| StoreError::Unknown(anyhow!("failed to convert {} to BigInt: {}", i, e)))
    }

    fn from_bytes(b: &str) -> Result<Self, StoreError> {
        scalar::Bytes::from_str(b)
            .map(Value::Bytes)
            .map_err(|e| StoreError::Unknown(anyhow!("failed to convert {} to Bytes: {}", b, e)))
    }

    fn from_vec(v: Vec<Self>) -> Self {
        Value::List(v)
    }
}
impl FromEntityData for Entity {
    type Value = Value;

    fn insert_entity_data(&mut self, key: String, v: Value) {
        self.insert(key, v);
    }
}

fn str_as_bytes(id: &str) -> QueryResult<scalar::Bytes> {
    scalar::Bytes::from_str(&id).map_err(|e| DieselError::SerializationError(Box::new(e)))
}
/// Convert Postgres string representation of bytes "\xdeadbeef"
/// to ours of just "deadbeef".
fn bytes_as_str(id: &str) -> String {
    id.trim_start_matches("\\x").to_owned()
}
