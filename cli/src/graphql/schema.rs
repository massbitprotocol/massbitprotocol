use super::s::Document;
use anyhow::Error;

#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    pub document: Document,
}

impl Schema {
    pub fn parse(raw: &str) -> Result<Self, Error> {
        let document = graphql_parser::parse_schema(&raw)?.into_static();
        let schema = Schema { document };
        Ok(schema)
    }
}
