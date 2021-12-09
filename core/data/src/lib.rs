pub mod graphql;
pub mod log;
pub mod metrics;
pub mod query;
pub mod schema;
pub mod store;
pub mod utils;
pub mod prelude {
    pub use crate::graphql::*;
    pub use crate::query::*;
    pub use crate::store::{IndexerStore, StoreError, StoreEventStreamBox};
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
        Document, Value, OperationDefinition, InlineFragment, TypeCondition,
        FragmentSpread, Field, Selection, SelectionSet, FragmentDefinition,
        Directive, VariableDefinition, Type,
    });
    static_graphql!(s, schema, {
        Field, Directive, InterfaceType, ObjectType, Value, TypeDefinition,
        EnumType, Type, Document, ScalarType, InputValue, DirectiveDefinition,
        UnionType, InputObjectType, EnumValue,
    });
}

pub use massbit_common::util::task_spawn;
