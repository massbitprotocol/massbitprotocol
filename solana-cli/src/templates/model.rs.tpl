use crate::STORE;
use crate::{Entity, EntityFilter, EntityOrder, EntityRange, Value};
use crate::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
pub use massbit_drive::{FromEntity, ToMap};
use std::collections::HashMap;

{%- for name, entity in entities %}
#[derive(Default, Debug, Clone, FromEntity, ToMap)]
{{ entity }}

impl Into<Entity> for {{ name }} {
    fn into(self) -> Entity {
        let map = {{ name }}::to_map(self.clone());
        Entity::from(map)
    }
}
impl {{ name }} {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("{{ name }}".to_string(), self.clone().into());
        }
    }
    pub fn get(entity_id: &String) -> Option<{{ name }}> {
        unsafe {
            let entity = STORE
                .as_mut()
                .unwrap()
                .get("{{ name }}".to_string(), entity_id);
            match entity {
                Some(e) => Some({{ name }}::from_entity(&e)),
                None => None,
            }
        }
    }
    pub fn query(
        filter: Option<EntityFilter>,
        order: EntityOrder,
        range: EntityRange,
    ) -> Vec<{{ name }}> {
        unsafe {
            STORE
                .as_ref()
                .unwrap()
                .query("{{ name }}".to_string(), filter, order, range)
                .iter()
                .map(|e| {{ name }}::from_entity(e))
                .collect::<Vec<{{ name }}>>()
        }
    }
}

{%- endfor -%}
