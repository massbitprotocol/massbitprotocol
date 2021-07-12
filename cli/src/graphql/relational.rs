use super::ext::DocumentExt;
use super::schema::Schema;
use super::{q, s};
use anyhow::{anyhow, Error};
use inflector::Inflector;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::{self, Write};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum ValueType {
    Boolean,
    BigInt,
    Bytes,
    BigDecimal,
    Int,
    String,
}

impl FromStr for ValueType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Boolean" => Ok(ValueType::Boolean),
            "BigInt" => Ok(ValueType::BigInt),
            "Bytes" => Ok(ValueType::Bytes),
            "BigDecimal" => Ok(ValueType::BigDecimal),
            "Int" => Ok(ValueType::Int),
            "String" | "ID" => Ok(ValueType::String),
            s => Err(anyhow!("Type not available in this context: {}", s)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum FieldType {
    Boolean,
    Int,
    String,
}

impl FieldType {
    pub fn from_field_type(field_type: &q::Type) -> Result<FieldType, Error> {
        let name = named_type(field_type);
        match ValueType::from_str(name)? {
            ValueType::Boolean => Ok(FieldType::Boolean),
            ValueType::Int | ValueType::BigInt => Ok(FieldType::Int),
            ValueType::String => Ok(FieldType::String),
            _ => Err(anyhow!("Invalid field type")),
        }
    }

    pub fn rust_type(&self) -> &str {
        match self {
            FieldType::Boolean => "bool",
            FieldType::Int => "i64",
            FieldType::String => "String",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
}

impl Field {
    fn new(field: &s::Field) -> Result<Field, Error> {
        let name = (&*field.name).to_snake_case();
        let field_type = FieldType::from_field_type(&field.field_type)?;
        Ok(Field { name, field_type })
    }

    fn rust_type(&self) -> &str {
        self.field_type.rust_type()
    }

    fn as_rust(&self, out: &mut String) -> fmt::Result {
        write!(out, "pub {}: {}", self.name, self.rust_type())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Model {
    pub name: String,
    pub fields: Vec<Field>,
}

impl Model {
    pub fn new(obj: &s::ObjectType) -> Result<Model, Error> {
        let name = (*obj.name).to_string();
        let fields = obj
            .fields
            .iter()
            .map(|field| Field::new(field))
            .collect::<Result<Vec<Field>, Error>>()?;
        let model = Model { name, fields };
        Ok(model)
    }

    pub fn as_rust_struct(&self, out: &mut String) -> fmt::Result {
        writeln!(out, "pub struct {} {{", self.name)?;
        for field in self.fields.iter() {
            write!(out, "    ")?;
            field.as_rust(out)?;
            writeln!(out, ",")?;
        }
        write!(out, "}}")
    }
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub models: HashMap<String, Model>,
}

impl Layout {
    pub fn new(schema: &Schema) -> Result<Self, Error> {
        let object_types = schema
            .document
            .get_object_type_definitions()
            .into_iter()
            .collect::<Vec<_>>();

        let models = object_types
            .iter()
            .enumerate()
            .map(|(_, obj_type)| Model::new(obj_type))
            .collect::<Result<Vec<_>, _>>()?;

        let models: HashMap<_, _> = models
            .into_iter()
            .fold(HashMap::new(), |mut models, model| {
                models.insert(model.name.clone(), model);
                models
            });

        Ok(Layout { models })
    }
}

fn named_type(field_type: &q::Type) -> &str {
    match field_type {
        q::Type::NamedType(name) => name.as_str(),
        q::Type::ListType(child) => named_type(child),
        q::Type::NonNullType(child) => named_type(child),
    }
}
