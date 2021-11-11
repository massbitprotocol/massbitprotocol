use crate::generator::helper::replace_invalid_identifier_chars;
use inflector::Inflector;
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
    pub offset: Option<u16>,
    #[serde(rename = "variantTag")]
    pub variant_tag: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accounts: Option<Vec<AccountInfo>>,
}

impl Variant {
    pub fn get_size(&self) -> Option<usize> {
        self.inner_type.as_ref().and_then(|typ| match typ.as_str() {
            "i8" | "u8" => Some(1),
            "i16" | "u16" => Some(2),
            "i32" | "u32" => Some(4),
            "i64" | "u64" => Some(8),
            "i128" | "u128" => Some(16),
            type_name => None,
        })
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountInfo {
    pub index: usize,
    pub name: String,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Property {
    pub name: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
    #[serde(rename = "arrayLength")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_length: Option<usize>,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
impl Property {
    pub fn size(&self) -> usize {
        match self.length {
            None => match self.data_type.as_str() {
                "u8" | "i8" => 1_usize,
                "u16" | "i16" => 2_usize,
                "u32" | "i32" => 4_usize,
                "u64" | "i64" => 8_usize,
                "u128" | "i128" => 16_usize,
                &_ => 1_usize,
            },
            Some(len) => len,
        }
    }
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
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize, Default)]
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
    pub offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "variantTagLength")]
    pub variant_tag_length: Option<usize>,
}

impl Schema {
    pub fn get_pascal_name(&self, name: &String) -> String {
        replace_invalid_identifier_chars(&name.as_str().to_pascal_case())
    }
    //Get size of struct type
    pub fn get_size(&self) -> Option<usize> {
        self.properties.as_ref().and_then(|properties| {
            let mut size = 0usize;
            for property in properties {
                size = size + property.size();
            }
            Some(size)
        })
    }
}
