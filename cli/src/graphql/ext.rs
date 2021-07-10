use super::s::{Definition, Document, ObjectType, TypeDefinition};

pub trait DocumentExt {
    fn get_object_type_definitions(&self) -> Vec<&ObjectType>;
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
}
