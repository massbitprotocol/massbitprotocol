extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate inflector;

use inflector::Inflector;
use proc_macro::TokenStream;
use quote::Tokens;
use syn::{Attribute, DeriveInput, Field, Lit, MetaItem, NestedMetaItem};

#[proc_macro_derive(Table, attributes(table_name, column_name, primary_key, column_type))]
pub fn table(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_table(&ast);
    gen.parse().unwrap()
}

fn impl_table(ast: &syn::DeriveInput) -> Tokens {
    let table = Table::from_ast(ast);
    table.to_tokens()
}

fn attr_name(attr: &syn::Attribute) -> &str {
    match &attr.value {
        &MetaItem::Word(ref id) => id.as_ref(),
        &MetaItem::List(ref id, _) => id.as_ref(),
        &MetaItem::NameValue(ref id, _) => id.as_ref(),
    }
}

fn attr_by_name<'a>(name: &str, attrs: &'a Vec<syn::Attribute>) -> Option<&'a Attribute> {
    attrs.iter().find(|a| attr_name(a) == name)
}

fn attr_is_word(attr: &syn::Attribute, word: Option<&str>) -> bool {
    match &attr.value {
        &MetaItem::Word(ref id) => match word {
            Some(w) => id == w,
            None => true,
        },
        _ => false,
    }
}

fn attr_lit(attr: &syn::Attribute) -> Option<&Lit> {
    match &attr.value {
        &MetaItem::NameValue(_, ref lit) => Some(lit),
        _ => None,
    }
}

fn attr_lit_str(attr: &syn::Attribute) -> Option<&str> {
    attr_lit(attr).and_then(|lit| match lit {
        &Lit::Str(ref s, _) => Some(s.as_ref()),
        _ => None,
    })
}

fn attr_list(attr: &syn::Attribute) -> Option<&Vec<NestedMetaItem>> {
    match &attr.value {
        &MetaItem::List(_, ref items) => Some(items),
        _ => None,
    }
}

fn attr_list_str(attr: &syn::Attribute) -> Option<Vec<&str>> {
    attr_list(attr).and_then(|items| {
        let mut vals = Vec::new();

        for nesteditem in items {
            match nesteditem {
                &NestedMetaItem::MetaItem(ref item) => match item {
                    &MetaItem::Word(ref ident) => {
                        vals.push(ident.as_ref());
                    }
                    _ => return None,
                },
                _ => return None,
            }
        }

        Some(vals)
    })
}

fn body_as_struct_fields(body: &syn::Body) -> Option<&Vec<Field>> {
    match body {
        &syn::Body::Struct(ref data) => match data {
            &syn::VariantData::Struct(ref fields) => Some(fields),
            _ => None,
        },
        _ => None,
    }
}

struct Column {
    field_name: String,
    column_name: String,
    column_type: syn::Ty,
}

impl Column {
    fn from_field(field: &Field) -> Option<Column> {
        if attr_by_name("skip", &field.attrs).is_some() {
            // Skip attribute given, so skip this field.
            return None;
        }

        let field_name = field.ident.as_ref().unwrap().to_string();

        let column_name = match attr_by_name("column_name", &field.attrs) {
            Some(attr) => match attr_lit_str(attr) {
                Some(s) => s.trim().to_string(),
                None => {
                    panic!("Attribute #column_name on field {:?} has incorrect format - Expected #[column_name=\"name\"",
                           field.ident);
                }
            },
            None => {
                // No explicit column name given, so just use the field name.
                field_name.clone()
            }
        };

        let column_type = match attr_by_name("column_type", &field.attrs) {
            Some(attr) => match attr_lit_str(attr) {
                Some(s) => match syn::parse_type(s) {
                    Ok(toks) => toks,
                    Err(e) => {
                        panic!("Could not parse type \"{}\" in #column_type attribute on field {:?}: {:?}",
                               s, field.ident, e);
                    }
                },
                None => {
                    panic!("Attribute #column_type on field {:?} has incorrect format - expected #[column_type=\"SomeType\"]",
                           field.ident);
                }
            },
            None => {
                panic!("Missing required attribute #column_type on field {:?} - expected #[column_type=\"SomeType\"]",
                       field.ident);
            }
        };

        Some(Column {
            field_name,
            column_name,
            column_type,
        })
    }
}

struct Table {
    name: String,
    primary_keys: Vec<String>,
    columns: Vec<Column>,
}

impl Table {
    fn from_ast(ast: &syn::DeriveInput) -> Self {
        let tbl_name = match attr_by_name("table_name", &ast.attrs) {
            Some(attr) => match attr_lit_str(attr) {
                Some(s) => s.trim().to_string(),
                None => {
                    panic!("Attribute table_name has incorrect format - Expected #[table_name=\"name\"")
                }
            },
            None => {
                // No table name attribute, so auto-generate it.
                let snaked = ast.ident.as_ref().to_snake_case();
                snaked.to_plural()
            }
        };

        let mut default_primary_key = false;
        let primary_keys = match attr_by_name("primary_key", &ast.attrs) {
            Some(attr) => match attr_list_str(attr) {
                Some(values) => values.into_iter().map(|v| v.trim().to_string()).collect(),
                None => {
                    panic!("Attribute has invalid format - expected #[primary_key(a,b)]")
                }
            },
            None => {
                default_primary_key = true;
                vec!["id".to_string()]
            }
        };

        let fields = match body_as_struct_fields(&ast.body) {
            Some(fields) => fields,
            None => {
                panic!("#[derive(Table)] can only be used on structs with fields");
            }
        };

        let columns: Vec<Column> = fields.into_iter().filter_map(Column::from_field).collect();

        // Validate primary keys.
        for key in &primary_keys {
            if columns
                .iter()
                .find(|c| c.column_name.as_str() == *key)
                .is_none()
            {
                if default_primary_key {
                    panic!("Tables must have an 'id' column or an explicitly specified primary key: #[primary_key(a)]");
                } else {
                    panic!("Table has primary key {} specified, but no such f")
                }
            }
        }

        let tbl = Table {
            name: tbl_name,
            primary_keys,
            columns,
        };

        tbl
    }

    fn to_tokens(self) -> Tokens {
        let name = syn::Ident::new(self.name);

        let mut primary_keys = Tokens::new();

        primary_keys.append_separated(
            self.primary_keys.into_iter().map(|k| syn::Ident::new(k)),
            ",",
        );

        let columns = self.columns.into_iter().map(|c| {
            let name = syn::Ident::new(c.column_name);
            let typ = c.column_type;
            quote! {
                #name -> #typ,
            }
        });

        quote! {
            table!{
                #name(#primary_keys) {
                    #(#columns)*
                }
            }
        }
    }
}
