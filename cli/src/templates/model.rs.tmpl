use crate::STORE;
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

{%- for name, entity in entities %}

#[derive(Default, Clone, FromMap, ToMap)]
{{ entity }}

impl Into<structmap::GenericMap> for {{ name }} {
    fn into(self) -> structmap::GenericMap {
        {{ name }}::to_genericmap(self.clone())
    }
}

impl {{ name }} {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_ref()
                .unwrap()
                .save("{{ name }}".to_string(), self.clone().into());
        }
    }
}
{%- endfor -%}