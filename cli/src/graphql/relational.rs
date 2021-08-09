use super::ext::{DocumentExt, ObjectTypeExt};
use super::schema::Schema;
use super::{q, s};
use anyhow::{anyhow, Error};
use graphql_parser::schema;
use inflector::Inflector;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::convert::TryFrom;
use std::fmt::{self, Write};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityType(String);

impl EntityType {
    pub fn new(entity_type: String) -> Self {
        Self(entity_type)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> From<&schema::ObjectType<'a, String>> for EntityType {
    fn from(object_type: &schema::ObjectType<'a, String>) -> Self {
        EntityType::new(object_type.name.to_owned())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum IdType {
    String,
}

impl TryFrom<&s::ObjectType> for IdType {
    type Error = Error;

    fn try_from(obj_type: &s::ObjectType) -> Result<Self, Self::Error> {
        let pk = obj_type
            .field(&PRIMARY_KEY_COLUMN.to_owned())
            .expect("Each ObjectType has an `id` field");
        Self::try_from(&pk.field_type)
    }
}

impl TryFrom<&s::Type> for IdType {
    type Error = Error;

    fn try_from(field_type: &s::Type) -> Result<Self, Self::Error> {
        let name = named_type(field_type);

        match ValueType::from_str(name)? {
            ValueType::String => Ok(IdType::String),
            _ => Err(anyhow!(
                "The `id` field has type `{}` but only `String`, `Bytes`, and `ID` are allowed",
                &name
            )
            .into()),
        }
    }
}

type IdTypeMap = HashMap<EntityType, IdType>;

type EnumMap = BTreeMap<String, Arc<BTreeSet<String>>>;

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

impl ValueType {
    /// Return `true` if `s` is the name of a builtin scalar type
    pub fn is_scalar(s: &str) -> bool {
        Self::from_str(s).is_ok()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum ModelFieldType {
    Boolean,
    Int,
    String,
}

impl From<IdType> for ModelFieldType {
    fn from(id_type: IdType) -> Self {
        match id_type {
            IdType::String => ModelFieldType::String,
        }
    }
}

impl ModelFieldType {
    pub fn from_field_type(
        field_type: &q::Type,
        enums: &EnumMap,
        id_types: &IdTypeMap,
    ) -> Result<ModelFieldType, Error> {
        let name = named_type(field_type);

        if let Some(id_type) = id_types.get(&EntityType::new(name.to_string())) {
            return Ok(id_type.clone().into());
        }

        if let Some(_values) = enums.get(&*name) {
            return Ok(ModelFieldType::String);
        }

        match ValueType::from_str(name)? {
            ValueType::Boolean => Ok(ModelFieldType::Boolean),
            ValueType::Int | ValueType::BigInt | ValueType::BigDecimal => Ok(ModelFieldType::Int),
            ValueType::String => Ok(ModelFieldType::String),
            _ => Err(anyhow!("Invalid field type")),
        }
    }

    pub fn rust_type(&self) -> &str {
        match self {
            ModelFieldType::Boolean => "bool",
            ModelFieldType::Int => "i64",
            ModelFieldType::String => "String",
        }
    }
}

/// The name for the primary key column of a table; hardcoded for now
pub(crate) const PRIMARY_KEY_COLUMN: &str = "id";

#[derive(Debug, Clone, Serialize)]
pub struct ModelField {
    pub name: String,
    pub field_type: ModelFieldType,
}

impl ModelField {
    fn new(field: &s::Field, enums: &EnumMap, id_types: &IdTypeMap) -> Result<ModelField, Error> {
        let name = (&*field.name).to_snake_case();
        let field_type = ModelFieldType::from_field_type(&field.field_type, enums, id_types)?;
        Ok(ModelField { name, field_type })
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
    pub fields: Vec<ModelField>,
}

impl Model {
    pub fn new(
        defn: &s::ObjectType,
        enums: &EnumMap,
        id_types: &IdTypeMap,
    ) -> Result<Model, Error> {
        let name = (*defn.name).to_string();
        let fields = defn
            .fields
            .iter()
            .map(|field| ModelField::new(field, enums, id_types))
            .collect::<Result<Vec<ModelField>, Error>>()?;
        let model = Model { name, fields };
        Ok(model)
    }

    pub fn as_rust(&self, out: &mut String) -> fmt::Result {
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
        let enums: EnumMap = schema
            .document
            .get_enum_definitions()
            .iter()
            .map(
                |enum_type| -> Result<(String, Arc<BTreeSet<String>>), Error> {
                    Ok((
                        enum_type.name.clone(),
                        Arc::new(
                            enum_type
                                .values
                                .iter()
                                .map(|value| value.name.to_owned())
                                .collect::<BTreeSet<_>>(),
                        ),
                    ))
                },
            )
            .collect::<Result<_, _>>()?;

        let object_types = schema
            .document
            .get_object_type_definitions()
            .into_iter()
            .collect::<Vec<_>>();

        // Map of type name to the type of the ID column for the object_types
        // and interfaces in the schema
        let id_types = object_types
            .iter()
            .map(|obj_type| IdType::try_from(*obj_type).map(|t| (EntityType::from(*obj_type), t)))
            .collect::<Result<IdTypeMap, _>>()?;

        let models = object_types
            .iter()
            .enumerate()
            .map(|(_, obj_type)| Model::new(obj_type, &enums, &id_types))
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

fn is_object_type(field_type: &q::Type, enums: &EnumMap) -> bool {
    let name = named_type(field_type);

    !enums.contains_key(&*name) && !ValueType::is_scalar(name)
}
