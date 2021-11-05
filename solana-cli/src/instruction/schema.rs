use crate::instruction::one_or_many;
pub type PositiveInteger = i64;
pub type PositiveIntegerDefault0 = serde_json::Value;
pub type SchemaArray = Vec<Schema>;
pub type VariantArray = Vec<Variant>;
pub type PropertyArray = Vec<Property>;
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Variant {
    pub name: String,
    pub value: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "innerName")]
    pub inner_name: Option<String>,
    #[serde(rename = "innerType")]
    pub inner_type: Option<String>,
    #[serde(rename = "innerScope")]
    pub inner_scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    //For unpacking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(rename = "variantTag")]
    pub variant_tag: i32,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Property {
    pub name: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    pub length: u32,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(rename = "variantTag")]
pub enum VariantTag {
    #[serde(rename = "u8")]
    U8(u8),
    #[serde(rename = "u16")]
    U16(u16),
    #[serde(rename = "u32")]
    U32(u32),
    #[serde(rename = "u64")]
    U64(u64),
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(rename = "simpleTypes")]
pub enum SimpleTypes {
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
}
pub type StringArray = Vec<String>;
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variants: Option<VariantArray>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub properties: Option<PropertyArray>,
    #[serde(default)]
    pub definitions: ::std::collections::BTreeMap<String, Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "variantTagLength")]
    pub variant_tag_length: Option<u8>,
}
