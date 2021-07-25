use super::s::{Definition, Document, EnumType, Field, ObjectType, TypeDefinition};

pub trait ObjectTypeExt {
    fn field(&self, name: &str) -> Option<&Field>;
}

impl ObjectTypeExt for ObjectType {
    fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|field| &field.name == name)
    }
}

pub trait DocumentExt {
    fn get_object_type_definitions(&self) -> Vec<&ObjectType>;
    fn get_enum_definitions(&self) -> Vec<&EnumType>;
}

impl DocumentExt for Document {
    fn get_object_type_definitions(&self) -> Vec<&ObjectType> {
        self.definitions
            .iter()
            .filter_map(|d| match d {
                Definition::TypeDefinition(TypeDefinition::Object(t)) => Some(t),
                _ => None,
            })
            .collect()
    }

    fn get_enum_definitions(&self) -> Vec<&EnumType> {
        self.definitions
            .iter()
            .filter_map(|d| match d {
                Definition::TypeDefinition(TypeDefinition::Enum(e)) => Some(e),
                _ => None,
            })
            .collect()
    }
}
