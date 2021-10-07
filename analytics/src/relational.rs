use graph::data::schema::FulltextConfig;
use graph::prelude::s::EnumType;
use graph::prelude::StoreError;
use inflector::Inflector;
use std::fmt;

/// A string we use as a SQL name for a table or column. The important thing
/// is that SQL names are snake cased. Using this type makes it easier to
/// spot cases where we use a GraphQL name like 'bigThing' when we should
/// really use the SQL version 'big_thing'
///
/// We use `SqlName` for example for table and column names, and feed these
/// directly to Postgres. Postgres truncates names to 63 characters; if
/// users have GraphQL type names that do not differ in the first
/// 63 characters after snakecasing, schema creation will fail because, to
/// Postgres, we would create the same table twice. We consider this case
/// to be pathological and so unlikely in practice that we do not try to work
/// around it in the application.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct SqlName(String);

impl SqlName {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn quoted(&self) -> String {
        format!("\"{}\"", self.0)
    }

    // Check that `name` matches the regular expression `/[A-Za-z][A-Za-z0-9_]*/`
    // without pulling in a regex matcher
    pub fn check_valid_identifier(name: &str, kind: &str) -> Result<(), StoreError> {
        let mut chars = name.chars();
        match chars.next() {
            Some(c) => {
                if !c.is_ascii_alphabetic() && c != '_' {
                    let msg = format!(
                        "the name `{}` can not be used for a {}; \
                         it must start with an ASCII alphabetic character or `_`",
                        name, kind
                    );
                    return Err(StoreError::InvalidIdentifier(msg));
                }
            }
            None => {
                let msg = format!("can not use an empty name for a {}", kind);
                return Err(StoreError::InvalidIdentifier(msg));
            }
        }
        for c in chars {
            if !c.is_ascii_alphanumeric() && c != '_' {
                let msg = format!(
                    "the name `{}` can not be used for a {}; \
                     it can only contain alphanumeric characters and `_`",
                    name, kind
                );
                return Err(StoreError::InvalidIdentifier(msg));
            }
        }
        Ok(())
    }

    pub fn verbatim(s: String) -> Self {
        SqlName(s)
    }

    // pub fn qualified_name(namespace: &Namespace, name: &SqlName) -> Self {
    //     SqlName(format!("\"{}\".\"{}\"", namespace, name.as_str()))
    // }
}

impl From<&str> for SqlName {
    fn from(name: &str) -> Self {
        SqlName(name.to_snake_case())
    }
}

impl From<String> for SqlName {
    fn from(name: String) -> Self {
        SqlName(name.to_snake_case())
    }
}

impl fmt::Display for SqlName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

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
    TSVector(FulltextConfig),
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
            ColumnType::TSVector(_) => "tsvector",
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
    pub alias: Option<&'a str>,
}

impl<'a> Table<'a> {
    pub fn new(name: &'a str, alias: Option<&'a str>) -> Self {
        Table {
            name: SqlName::from(name),
            alias,
        }
    }
}
