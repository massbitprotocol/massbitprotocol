use arrayref::{array_ref, array_refs};
use bytemuck::cast;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::num::*;

{%- for (name, schema) in definitions %}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
{{ name }}
