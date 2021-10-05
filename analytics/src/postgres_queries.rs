use crate::models::CommandData;
use crate::relational::{Column, ColumnType, SqlName, Table};
use crate::sql_value::SqlValue;
use core::str::FromStr;
use graph::components::store::StoreError;
use graph::data::store::scalar;
use graph::prelude::{Entity, Value};
use massbit_common::prelude::diesel::pg::Pg;
use massbit_common::prelude::diesel::query_builder::{AstPass, QueryFragment, QueryId};
use massbit_common::prelude::diesel::result::Error as DieselError;
use massbit_common::prelude::diesel::sql_types::{Array, Binary, Bool, Integer, Text, Varchar};
use massbit_common::prelude::diesel::{
    insert_into, r2d2, sql_query, Connection, IntoSql, QueryResult, RunQueryDsl,
};

const PRIMARY_KEY_COLUMN: &str = "id";
#[derive(Debug)]
pub struct UpsertQuery<'a> {
    table: &'a Table<'a>,
    entities: &'a Vec<Entity>,
    columns: &'a Vec<Column>,
    conflict_fragment: &'a Option<UpsertConflictFragment<'a>>,
}
impl<'a> UpsertQuery<'a> {
    pub fn new(
        table: &'a Table,
        columns: &'a Vec<Column>,
        entities: &'a Vec<Entity>,
        conflict_fragment: &'a Option<UpsertConflictFragment<'a>>,
    ) -> Result<UpsertQuery<'a>, StoreError> {
        Ok(UpsertQuery {
            table,
            entities,
            columns,
            conflict_fragment,
        })
    }
}
impl<'a> From<&CommandData<'a>> for UpsertQuery<'a> {
    fn from(cmd: &CommandData<'a>) -> Self {
        UpsertQuery {
            table: cmd.table,
            entities: cmd.values,
            columns: cmd.columns,
            conflict_fragment: cmd.conflict_fragment,
        }
    }
}
impl<'a> QueryFragment<Pg> for UpsertQuery<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        // Construct a query
        //   insert into schema.table as t (column, ...)
        //   values
        //   (a, b, c),
        //   (d, e, f)
        //   [...]
        //   (x, y, z)
        //   on conflict (name)
        //   do
        //   update set value = t.value + EXCLUDED.value;
        //
        // and convert and bind the entity's values into it
        out.push_sql("insert into ");
        out.push_sql(self.table.name.as_str());
        if self.table.alias.is_some() {
            out.push_sql(" as ");
            out.push_sql(self.table.alias.unwrap());
        }
        out.push_sql("(");
        // Use a `Peekable` iterator to help us decide how to finalize each line.
        //let mut col_iter = self.columns.iter().map(|col| col).peekable();
        let mut col_iter = self.columns.iter().peekable();
        while let Some(column) = col_iter.next() {
            out.push_identifier(column.name.as_str());
            //Still has column
            if col_iter.peek().is_some() {
                out.push_sql(", ");
            }
        }

        //out.push_identifier(BLOCK_RANGE_COLUMN)?;

        out.push_sql(") values\n");

        // Use a `Peekable` iterator to help us decide how to finalize each line.
        //let mut iter = self.entities.iter().map(|entity| entity).peekable();
        let mut iter = self.entities.iter().peekable();
        while let Some(entity) = iter.next() {
            out.push_sql("(");
            //let mut col_iter = self.columns.iter().map(|col| col).peekable();
            let mut col_iter = self.columns.iter().peekable();
            while let Some(column) = col_iter.next() {
                // If the column name is not within this entity's fields, we will issue the
                // null value in its place
                if let Some(value) = entity.get(column.name.as_str()) {
                    QueryValue(value, &column.column_type)
                        .walk_ast(out.reborrow())
                        .unwrap();
                } else {
                    out.push_sql("null");
                }
                //Still has column
                if col_iter.peek().is_some() {
                    out.push_sql(", ");
                }
            }
            out.push_sql(")");
            // finalize line according to remaining entities to insert
            if iter.peek().is_some() {
                out.push_sql(",\n");
            }
        }
        if self.conflict_fragment.is_some() {
            self.conflict_fragment
                .as_ref()
                .walk_ast(out.reborrow())
                .unwrap();
        }
        Ok(())
    }
}

impl<'a> QueryId for UpsertQuery<'a> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}
impl<'a, Conn> RunQueryDsl<Conn> for UpsertQuery<'a> {}

#[derive(Debug, Clone)]
pub struct UpdateExpression<'a> {
    field: &'a str,
    expression: &'a str,
}
impl<'a> UpdateExpression<'a> {
    pub fn new(field: &'a str, expression: &'a str) -> Self {
        UpdateExpression { field, expression }
    }
}
#[derive(Debug, Clone)]
pub struct UpsertConflictFragment<'a> {
    constraint: &'a str,
    expressions: Vec<UpdateExpression<'a>>,
}
impl<'a> UpsertConflictFragment<'a> {
    pub fn new(constraint: &'a str) -> Self {
        UpsertConflictFragment {
            constraint,
            expressions: Vec::default(),
        }
    }
    pub fn add_expression(&mut self, field: &'a str, expression: &'a str) -> &mut Self {
        self.expressions
            .push(UpdateExpression { field, expression });
        self
    }
}
impl<'a> QueryFragment<Pg> for UpsertConflictFragment<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        // Construct on conflict fragment
        //   on conflict (name)
        //   do
        //   update set field = expression;
        out.push_sql("\non conflict ON CONSTRAINT ");
        out.push_sql(self.constraint);
        if self.expressions.len() == 0 {
            out.push_sql(" DO NOTHING ");
        } else {
            out.push_sql(" DO UPDATE SET ");
            let mut peeable_iter = self.expressions.iter().map(|exp| exp).peekable();
            while let Some(exp) = peeable_iter.next() {
                out.push_sql(exp.field);
                out.push_sql("=");
                out.push_sql(exp.expression);
                //Still has expression
                if peeable_iter.peek().is_some() {
                    out.push_sql(", ");
                }
            }
        }

        Ok(())
    }
}
/// A `QueryValue` makes it possible to bind a `Value` into a SQL query
/// using the metadata from Column
struct QueryValue<'a>(&'a Value, &'a ColumnType);

impl<'a> QueryFragment<Pg> for QueryValue<'a> {
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        let column_type = self.1;

        match self.0 {
            Value::String(s) => match &column_type {
                ColumnType::String => out.push_bind_param::<Text, _>(s),
                ColumnType::Varchar => out.push_bind_param::<Varchar, _>(s),
                ColumnType::Enum(enum_type) => {
                    out.push_bind_param::<Text, _>(s)?;
                    out.push_sql("::");
                    out.push_sql(enum_type.name.as_str());
                    Ok(())
                }
                ColumnType::TSVector(_) => {
                    out.push_sql("to_tsquery(");
                    out.push_bind_param::<Text, _>(s)?;
                    out.push_sql(")");
                    Ok(())
                }
                ColumnType::Bytes | ColumnType::BytesId => {
                    let bytes = scalar::Bytes::from_str(&s)
                        .map_err(|e| DieselError::SerializationError(Box::new(e)))?;
                    out.push_bind_param::<Binary, _>(&bytes.as_slice())
                }
                _ => unreachable!(
                    "only string, enum and tsvector columns have values of type string"
                ),
            },
            Value::Int(i) => out.push_bind_param::<Integer, _>(i),
            Value::BigDecimal(d) => {
                out.push_bind_param::<Text, _>(&d.to_string())?;
                out.push_sql("::numeric");
                Ok(())
            }
            Value::Bool(b) => out.push_bind_param::<Bool, _>(b),
            Value::List(values) => {
                let sql_values = SqlValue::new_array(values.clone());
                match &column_type {
                    ColumnType::BigDecimal | ColumnType::BigInt => {
                        let text_values: Vec<_> = values.iter().map(|v| v.to_string()).collect();
                        out.push_bind_param::<Array<Text>, _>(&text_values)?;
                        out.push_sql("::numeric[]");
                        Ok(())
                    }
                    ColumnType::Boolean => out.push_bind_param::<Array<Bool>, _>(&sql_values),
                    ColumnType::Bytes => out.push_bind_param::<Array<Binary>, _>(&sql_values),
                    ColumnType::Int => out.push_bind_param::<Array<Integer>, _>(&sql_values),
                    ColumnType::String => out.push_bind_param::<Array<Text>, _>(&sql_values),
                    ColumnType::Varchar => out.push_bind_param::<Array<Varchar>, _>(&sql_values),
                    ColumnType::TextArray => out.push_bind_param::<Array<Text>, _>(&sql_values),
                    ColumnType::Enum(enum_type) => {
                        out.push_bind_param::<Array<Text>, _>(&sql_values)?;
                        out.push_sql("::");
                        out.push_sql(enum_type.name.as_str());
                        out.push_sql("[]");
                        Ok(())
                    }
                    // TSVector will only be in a Value::List() for inserts so "to_tsvector" can always be used here
                    ColumnType::TSVector(config) => {
                        if sql_values.is_empty() {
                            out.push_sql("''::tsvector");
                        } else {
                            out.push_sql("(");
                            for (i, value) in sql_values.iter().enumerate() {
                                if i > 0 {
                                    out.push_sql(") || ");
                                }
                                out.push_sql("to_tsvector(");
                                out.push_bind_param::<Text, _>(
                                    &config.language.as_str().to_string(),
                                )?;
                                out.push_sql("::regconfig, ");
                                out.push_bind_param::<Text, _>(&value)?;
                            }
                            out.push_sql("))");
                        }

                        Ok(())
                    }
                    ColumnType::BytesId => out.push_bind_param::<Array<Binary>, _>(&sql_values),
                }
            }
            Value::Null => {
                out.push_sql("null");
                Ok(())
            }
            Value::Bytes(b) => out.push_bind_param::<Binary, _>(&b.as_slice()),
            Value::BigInt(i) => {
                out.push_bind_param::<Text, _>(&i.to_string())?;
                out.push_sql("::numeric");
                Ok(())
            }
        }
    }
}
