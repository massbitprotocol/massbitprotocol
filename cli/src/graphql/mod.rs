pub mod ext;
pub mod relational;
pub mod schema;

macro_rules! static_graphql {
        ($m:ident, $m2:ident, {$($n:ident,)*}) => {
            pub mod $m {
                use graphql_parser::$m2 as $m;
                pub use $m::*;
                $(
                    pub type $n = $m::$n<'static, String>;
                )*
            }
        };
    }

static_graphql!(q, query, {
    Type,
});

static_graphql!(s, schema, {
    Definition, Document, ObjectType, TypeDefinition, Field,
});
