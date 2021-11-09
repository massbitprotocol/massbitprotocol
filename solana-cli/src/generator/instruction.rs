use crate::schema::{PropertyArray, Schema, VariantArray};
use std::fmt::Write;
impl Schema {
    pub fn gen_instruction(&self) -> String {
        let mut out = String::new();
        self.append_modules(&mut out);
        self.expand_definitions(&mut out);
        self.definitions.iter().map(|(name, def)| {});
        out
    }
    pub fn append_modules(&self, out: &mut String) {
        writeln!(
            out,
            r#"use bytemuck::cast;
use serde::{{Deserialize, Serialize}};
use solana_program::{{
    instruction::{{AccountMeta, Instruction}},
    pubkey::Pubkey,
    sysvar::rent,
}};
use std::convert::TryInto;

use arrayref::{{array_ref, array_refs}};
use num_enum::{{IntoPrimitive, TryFromPrimitive}};
use std::num::*;
"#
        );
    }
    pub fn expand_definitions(&self, out: &mut String) {
        if let Some(properties) = &self.properties {
            let fields = self.expand_fields(properties);
            let unpack = self.expand_struct_unpack();
            write! {
                out,
                "#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
                pub struct {struct_name} {{
                    {fields}
                }}
                impl {struct_name} {{
                    {unpack}
                }}",
                struct_name = &self.name.as_ref().unwrap().clone(),
                fields=fields,
                unpack=unpack
            };
        } else if let Some(variants) = &self.variants {
            let unpack = self.expand_enum_unpack();
            let variants = self.expand_variants(variants);
            write! {
                out,
                "#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
                pub enum {name} {{
                    {variants}
                }}
                impl {name} {{
                    {unpack}
                }}",
                name = &self.name.as_ref().unwrap().clone(),
                variants=variants,
                unpack=unpack
            };
        };
    }
    pub fn expand_fields(&self, properties: &PropertyArray) -> String {
        let mut res = String::new();
        res
    }
    pub fn expand_variants(&self, variants: &VariantArray) -> String {
        let mut res = String::new();

        res
    }
    pub fn expand_struct_unpack(&self) -> String {
        let mut res = String::new();
        res
    }
    pub fn expand_enum_unpack(&self) -> String {
        let mut res = String::new();
        res
    }
}
