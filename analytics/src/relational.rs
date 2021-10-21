use inflector::Inflector;
use massbit::prelude::s::EnumType;
use massbit::prelude::StoreError;
use massbit_store_postgres::relational::SqlName;
use std::fmt;

/// This is almost the same as graph::data::store::ValueType, but without
/// ID and List; with this type, we only care about scalar types that directly
/// correspond to Postgres scalar types
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnType {
    Boolean,
    BigDecimal,
    BigInt,
    Bytes,
    Int,
    String,
    Varchar,
    TextArray,
    //TSVector(FulltextConfig),
    Enum(EnumType),
    /// A `bytea` in SQL, represented as a ValueType::String; this is
    /// used for `id` columns of type `Bytes`
    BytesId,
}

impl ColumnType {
    pub fn sql_type(&self) -> &str {
        match self {
            ColumnType::Boolean => "boolean",
            ColumnType::BigDecimal => "numeric",
            ColumnType::BigInt => "numeric",
            ColumnType::Bytes => "bytea",
            ColumnType::Int => "integer",
            ColumnType::String => "text",
            ColumnType::Varchar => "varchar",
            ColumnType::TextArray => "text[]",
            //ColumnType::TSVector(_) => "tsvector",
            ColumnType::Enum(enum_type) => enum_type.name.as_str(),
            ColumnType::BytesId => "bytea",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Column {
    pub name: SqlName,
    pub column_type: ColumnType,
    is_reference: bool,
}

impl Column {
    pub fn new(field: &str, column_type: ColumnType) -> Column {
        let sql_name = SqlName::from(field);
        let is_reference = false;
        Column {
            name: sql_name,
            column_type,
            is_reference,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Table<'a> {
    pub name: SqlName,
    pub columns: Vec<Column>,
    pub alias: Option<&'a str>,
}

impl<'a> Table<'a> {
    pub fn new_with_alias(name: &'a str, columns: Vec<Column>, alias: Option<&'a str>) -> Self {
        Table {
            name: SqlName::from(name),
            columns,
            alias,
        }
    }
    pub fn new(name: &'a str, columns: Vec<Column>) -> Self {
        Table {
            name: SqlName::from(name),
            columns,
            alias: Some("t"),
        }
    }
}
