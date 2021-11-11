use crate::generator::helper::is_integer_type;
use crate::schema::{Property, PropertyArray, Schema, Variant, VariantArray};
use std::fmt::Write;

const modules: &str = r#"
use bytemuck::cast;
use serde::{{Deserialize, Serialize}};
use solana_program::{{
    pubkey::Pubkey,
    sysvar::rent,
}};
use arrayref::{{array_ref, array_refs}};
use num_enum::{{IntoPrimitive, TryFromPrimitive}};
use std::num::*;
"#;
impl Schema {
    pub fn gen_instruction(&self) -> String {
        let mut out = String::new();
        ///Import modules for instruction
        writeln!(out, "{}", modules);
        match &self.name {
            Some(name) => {
                self.expand_definitions(&mut out, name);
            }
            None => {}
        }
        out
    }
    pub fn expand_schema(&self, out: &mut String, name: &String, schema: &Schema) {
        schema.expand_definitions(out, name)
    }
    pub fn expand_definitions(&self, out: &mut String, schema_name: &String) {
        self.definitions.iter().for_each(|(name, def)| {
            self.expand_schema(out, name, def);
        });
        if let Some(properties) = &self.properties {
            let name = self.get_pascal_name(schema_name);
            let fields = self.expand_fields(properties);
            let unpack = self.expand_struct_unpack(&name);
            let struct_def = format!("pub struct {} {{{fields}}}", &name, fields = &fields);
            let struct_impl = format!("impl {} {{\n{unpack}\n}}", &name, unpack = &unpack);
            write! {
                out,
                "#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]\n{struct_def}\n{struct_impl}",
                struct_def = struct_def,
                struct_impl = struct_impl
            };
        } else if let Some(variants) = &self.variants {
            let name = self.get_pascal_name(schema_name);
            let variants = self.expand_variants(variants);
            let enum_def = format!(
                "pub enum {} {{\n{variants}\n}}",
                &name,
                variants = &variants
            );
            let unpack = self.expand_enum_unpack(&name);
            let enum_impl = format!("impl {} {{\n{unpack}\n}}", &name, unpack = &unpack);
            write! {
                out,
                "#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]\n{enum_def}\n{enum_impl}",
                enum_def = enum_def,
                enum_impl = enum_impl
            };
        };
    }
    pub fn expand_fields(&self, properties: &PropertyArray) -> String {
        properties
            .iter()
            .map(|property| {
                if property.array_length.is_some() && property.array_length.unwrap_or_default() > 0
                {
                    format!("pub {}: Vec<{}>", &property.name, &property.data_type)
                } else {
                    format!("pub {}:{}", &property.name, &property.data_type)
                }
            })
            .collect::<Vec<String>>()
            .join(",\n")
    }
    pub fn expand_variants(&self, variants: &VariantArray) -> String {
        variants
            .iter()
            .map(|variant| match &variant.inner_type {
                None => format!("{}", &variant.name),
                Some(inner) => {
                    format!("{}({})", &variant.name, inner)
                }
            })
            .collect::<Vec<String>>()
            .join(",\n")
    }
    pub fn expand_struct_unpack(&self, name: &String) -> String {
        let struct_size = self.properties.as_ref().and_then(|properties| {
            let mut total_size = 0_usize;
            for property in properties {
                total_size = total_size + property.size()
            }
            Some(total_size)
        });
        let mut ref_names: Vec<String> = Vec::default();
        let mut lengths: Vec<String> = Vec::default();
        //Store optional assigments
        let mut property_assignments: Vec<String> = Vec::default();
        //Collect is_some() condition
        let mut optional_conditions: Vec<String> = Vec::default();
        //Struct fields
        let mut properties: Vec<String> = Vec::default();
        for property in self.properties.as_ref().unwrap() {
            ref_names.push(format!("{}", &property.name));
            lengths.push(format!("{}", property.size()));
            //Expand struct field's data type.
            //Use unpack for user defined type other use try_from_primitive
            let field_value = self.expand_property_unpack(property);
            if self.is_option_value(property) {
                property_assignments.push(format!("let {} = {};", &property.name, &field_value));
                optional_conditions.push(format!("{}.is_some()", &property.name));
                properties.push(format!("{name}:{name}.unwrap()", name = &property.name));
            } else {
                properties.push(format!("{}: {}", &property.name, &field_value));
            }
        }
        let str_size = if let Some(val) = struct_size {
            format!("; {}", val)
        } else {
            String::from("")
        };
        let optional_flow = if optional_conditions.len() > 0 {
            (
                format!("if {} {{", optional_conditions.join("&&")),
                String::from(
                    r#"
                } else {
                    None
                }"#,
                ),
            )
        } else {
            (String::default(), String::default())
        };
        format!(
            r#"pub fn unpack(input: &[u8{size}]) -> Option<Self> {{
                let ({ref_names}) = array_refs![input, {lengths}];
                {assignments}
                {optional_if}
                Some({name} {{
                    {properties}
                }})
                {optional_else}
            }}"#,
            size = str_size,
            ref_names = ref_names.join(","),
            lengths = lengths.join(","),
            assignments = property_assignments.join("\n"),
            name = name,
            properties = properties.join(",\n"),
            optional_if = &optional_flow.0,
            optional_else = &optional_flow.1,
        )
    }
    pub fn expand_enum_unpack(&self, name: &String) -> String {
        let tag_len = self.variant_tag_length.unwrap_or(1);
        let separation = match self.offset {
            None => {
                format!(
                    "let (&tag_slice, data) = array_refs![input, {}; ..;];",
                    tag_len
                )
            }
            Some(offset) => {
                format!(
                    "let (&[offset], &tag_slice, data) = array_refs![input, {}, {}; ..;];",
                    offset, tag_len
                )
            }
        };
        let tag_val = match tag_len {
            1 => "let tag_val = u8::from_le_bytes(tag_slice) as u32;".to_string(),
            2 => "let tag_val = u16::from_le_bytes(tag_slice) as u32;".to_string(),
            _ => "let tag_val = u32::from_le_bytes(tag_slice) as u32;".to_string(),
        };
        let mut variants = self
            .variants
            .as_ref()
            .unwrap()
            .iter()
            .map(|variant| self.expand_variant_unpack(name, variant))
            .collect::<Vec<String>>();
        //Add remain pattern for enum unpacking;
        variants.push(String::from("_ => None"));
        let match_frag = format!("match tag_val {{{}}}", variants.join(",\n"));
        format!(
            r#"pub fn unpack(input: &[u8]) -> Option<Self> {{
                {separation}
                {tag_val}            
                {match_frag}
            }}"#,
            separation = separation,
            tag_val = tag_val,
            match_frag = match_frag
        )
    }
    pub fn expand_data_unpack(&self, field_name: &str, data_type: &str) -> String {
        if data_type.starts_with("NonZero") {
            let inner_type = &data_type[7..data_type.len()].to_lowercase();
            format!(
                "{}::new({}::from_le_bytes(*{}))",
                data_type, inner_type, &field_name
            )
        } else if is_integer_type(data_type) {
            format!("{}::from_le_bytes(*{})", data_type, field_name)
        } else {
            format!("{}::unpack({})", data_type, field_name)
        }
    }
    /// Return true if value of a property is optional
    /// A property can be none when unpacking if
    /// Property is not a vector, data_type is NonZero* or some user defined struct or enum
    ///
    pub fn is_option_value(&self, property: &Property) -> bool {
        let data_type = property.data_type.as_str();
        //Property is not vector
        property.array_length.unwrap_or_default() == 0 && !is_integer_type(data_type)
    }
    pub fn expand_property_unpack(&self, property: &Property) -> String {
        let data_type = property.data_type.as_str();
        match property.array_length {
            Some(val) => {
                let total_size = property.length.unwrap_or_default();
                if val > 0 && total_size > 0 {
                    //Size of a single vector element
                    let elm_size = total_size / val;
                    if elm_size * val < total_size {
                        panic!(format!("Error in property {}. Total size {} is not multiples of array length {}", &property.name, total_size, val))
                    } else {
                        let mut sizes = vec![];
                        let mut indexes = vec![];
                        for i in 0..val {
                            sizes.push(format!("{}", elm_size));
                            indexes.push(
                                self.expand_data_unpack(format!("arr.{}", i).as_str(), data_type),
                            );
                        }
                        format!(
                            r#"{{
                            let arr = array_refs![owner, {sizes}];
                            vec![{indexes}]
                        }}"#,
                            sizes = sizes.join(","),
                            indexes = indexes.join(",")
                        )
                    }
                } else {
                    String::from("Vec::default()")
                }
            }
            None => self.expand_data_unpack(property.name.as_str(), data_type),
        }
    }
    pub fn expand_variant_unpack(&self, name: &String, variant: &Variant) -> String {
        let var_tag = variant.variant_tag;
        match &variant.inner_type {
            Some(inner_type) => {
                let inner_schema = self.definitions.get(inner_type);
                let variant_size = inner_schema
                    .and_then(|schema| schema.get_size())
                    .or(variant.get_size());
                let (inner_value, field_slice) = match variant_size {
                    None => (
                        self.expand_data_unpack("data", inner_type.as_str()),
                        String::default(),
                    ),
                    Some(size) => (
                        self.expand_data_unpack("field_slice", inner_type.as_str()),
                        format!("let field_slice = array_ref![data, 0, {}];", size),
                    ),
                };
                if is_integer_type(inner_type.as_str()) {
                    format!(
                        r#"{var_tag} => {{
                                        {field_slice}
                                        Some({name}::{var_name}({inner_value}))
                                    }}"#,
                        var_tag = var_tag,
                        field_slice = field_slice,
                        name = name,
                        var_name = &variant.name,
                        inner_value = inner_value
                    )
                } else {
                    format!(
                        r#"{var_tag} => {{
                                        {field_slice}
                                        let inner = {inner_value};
                                        inner.and_then(|inner_val| Some({name}::{var_name}(inner_val)))
                                    }}"#,
                        var_tag = var_tag,
                        field_slice = field_slice,
                        name = name,
                        var_name = &variant.name,
                        inner_value = inner_value
                    )
                }
            }
            None => format!("{} => Some({}::{})", var_tag, name, &variant.name),
        }
    }
}
