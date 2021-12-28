use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct MetadataResource {
    #[serde(rename = "resource_version")]
    pub version: i32,
    pub metadata: Metadata,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub version: i32,
    pub sources: Vec<MetaSource>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MetaSource {
    pub kind: String,
    pub name: String,
    pub tables: Vec<Table>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Table {
    pub table: MetaTable,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MetaTable {
    pub name: String,
    pub schema: String,
}

impl MetaSource {
    pub fn filter(&mut self, schema: &str) -> &Self {
        self.tables.retain(|table| table.table.schema.eq(schema));
        self
    }
}

impl Metadata {
    pub fn filter(&mut self, schema: &str) -> &Self {
        self.sources.iter_mut().for_each(|source| {
            source.filter(schema);
        });
        self
    }
}
#[derive(Clone, Deserialize, Serialize)]
pub struct GraphqlSchemaResponse {
    pub data: GraphqlSchema,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct GraphqlSchema {
    #[serde(rename = "__schema")]
    pub schema: InnerSchema,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct InnerSchema {
    pub directives: Vec<serde_json::Value>,
    pub mutationType: NameType,
    pub queryType: NameType,
    #[serde(rename = "subscriptionType")]
    pub subscription_type: NameType,
    pub types: Vec<FieldType>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct NameType {
    pub name: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct FieldType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "enumValues")]
    pub enum_values: Option<Vec<EnumValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<Field>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "inputFields")]
    pub input_fields: Option<Vec<Field>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Vec<Interface>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "possibleTypes")]
    pub possible_types: Option<Vec<FieldType>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ofType")]
    pub of_type: Option<Box<FieldType>>,
}
impl FieldType {
    pub fn belong_to(&self, schema: &str) -> bool {
        match self.kind.as_ref().and_then(|k| Some(k.as_str())) {
            Some("OBJECT") => false,
            _ => true,
        }
    }
}
// #[derive(Clone, Deserialize, Serialize)]
// pub struct FieldType {
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub kind: Option<String>,
//     pub name: String,
//     #[serde(skip_serializing_if = "Option::is_none", rename = "ofType")]
//     pub of_type: Option<FieldType>,
// }
#[derive(Clone, Deserialize, Serialize)]
pub struct EnumValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecationReason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "isDeprecated")]
    pub is_deprecated: bool,
    pub name: String,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Field {
    #[serde(skip_serializing_if = "Option::is_none", rename = "defaultValue")]
    pub default_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "deprecationReason")]
    pub deprecation_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isDeprecated")]
    pub is_deprecated: Option<bool>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub field_type: Option<FieldType>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Interface {}

impl InnerSchema {
    pub fn filter(&mut self, schema_name: &str) -> &Self {
        self.types.retain(|elm| elm.belong_to(schema_name));
        self
    }
}
