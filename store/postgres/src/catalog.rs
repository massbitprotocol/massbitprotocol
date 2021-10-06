use diesel::pg::PgConnection;
use diesel::prelude::RunQueryDsl;
use diesel::OptionalExtension;
use diesel::{
    sql_types::{Nullable, Text},
    ExpressionMethods, QueryDsl,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use massbit::prelude::StoreError;

use crate::{
    primary::{Namespace, Site},
    relational::SqlName,
};

// Readonly; we only access the name
table! {
    pg_namespace(nspname) {
        nspname -> Text,
    }
}

table! {
    table_stats {
        id -> Integer,
        deployment -> Integer,
        table_name -> Text,
        is_account_like -> Nullable<Bool>,
    }
}

/// Information about what tables and columns we have in the database
#[derive(Debug, Clone)]
pub struct Catalog {
    pub site: Arc<Site>,
    text_columns: HashMap<String, HashSet<String>>,
}

impl Catalog {
    pub fn new(conn: &PgConnection, site: Arc<Site>) -> Result<Self, StoreError> {
        let text_columns = get_text_columns(conn, &site.namespace)?;
        Ok(Catalog { site, text_columns })
    }

    /// Return `true` if `table` exists and contains the given `column` and
    /// if that column is of data type `text`
    pub fn is_existing_text_column(&self, table: &SqlName, column: &SqlName) -> bool {
        self.text_columns
            .get(table.as_str())
            .map(|cols| cols.contains(column.as_str()))
            .unwrap_or(false)
    }
}

fn get_text_columns(
    conn: &PgConnection,
    namespace: &Namespace,
) -> Result<HashMap<String, HashSet<String>>, StoreError> {
    const QUERY: &str = "
        select table_name, column_name
          from information_schema.columns
         where table_schema = $1 and data_type = 'text'";

    #[derive(Debug, QueryableByName)]
    struct Column {
        #[sql_type = "Text"]
        pub table_name: String,
        #[sql_type = "Text"]
        pub column_name: String,
    }

    let map: HashMap<String, HashSet<String>> = diesel::sql_query(QUERY)
        .bind::<Text, _>(namespace.as_str())
        .load::<Column>(conn)?
        .into_iter()
        .fold(HashMap::new(), |mut map, col| {
            map.entry(col.table_name)
                .or_default()
                .insert(col.column_name);
            map
        });
    Ok(map)
}

pub(crate) mod table_schema {
    use super::*;

    /// The name and data type for the column in a table. The data type is
    /// in a form that it can be used in a `create table` statement
    pub struct Column {
        pub column_name: String,
        pub data_type: String,
    }

    #[derive(QueryableByName)]
    struct ColumnInfo {
        #[sql_type = "Text"]
        column_name: String,
        #[sql_type = "Text"]
        data_type: String,
        #[sql_type = "Text"]
        udt_name: String,
        #[sql_type = "Text"]
        udt_schema: String,
        #[sql_type = "Nullable<Text>"]
        elem_type: Option<String>,
    }

    impl From<ColumnInfo> for Column {
        fn from(ci: ColumnInfo) -> Self {
            // See description of `data_type` in
            // https://www.postgresql.org/docs/current/infoschema-columns.html
            let data_type = match ci.data_type.as_str() {
                "ARRAY" => format!(
                    "{}[]",
                    ci.elem_type.expect("array columns have an elem_type")
                ),
                "USER-DEFINED" => format!("{}.{}", ci.udt_schema, ci.udt_name),
                _ => ci.data_type.clone(),
            };
            Self {
                column_name: ci.column_name.clone(),
                data_type,
            }
        }
    }
}

pub fn account_like(conn: &PgConnection, site: &Site) -> Result<HashSet<String>, StoreError> {
    use table_stats as ts;
    let names = ts::table
        .filter(ts::deployment.eq(site.id))
        .select((ts::table_name, ts::is_account_like))
        .get_results::<(String, Option<bool>)>(conn)
        .optional()?
        .unwrap_or(vec![])
        .into_iter()
        .filter_map(|(name, account_like)| {
            if account_like == Some(true) {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    Ok(names)
}
