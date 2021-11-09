use arrayref::{array_ref, array_refs};
use bytemuck::cast;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::num::*;

{% for name, definition in definitions %}
{% if definition.is_struct %}
pub struct {{ name }} {

}
{% else %}
pub enum {{ name }} {

}
{% endif %}
{% endfor %}