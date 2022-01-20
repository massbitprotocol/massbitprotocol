use lazy_static::lazy_static;
use std::collections::HashMap;
lazy_static! {
    // https://www.codingame.com/playgrounds/365/getting-started-with-rust/primitive-data-types
    pub static ref PRIMITIVE_DATA_TYPES: Vec<&'static str> = vec![
        "bool", "char", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "i128", "u128", "isize", "usize", "f32",
        "f64", "str"
    ];

    // https://kotiri.com/2018/01/31/postgresql-diesel-rust-types.html
    pub static ref MAPPING_RUST_TYPES_TO_DB: HashMap<&'static str, &'static str> = HashMap::from([
        ("bool", "Bool"),
        //The graph generator postgres sql only handles with bigint
        ("i8", "Integer"),
        ("u8", "Integer"),
        ("i16", "Integer"),
        ("u16", "Integer"),
        ("i32", "Integer"),
        ("u32", "Integer"),
        ("NonZeroU8", "Integer"),
        ("NonZeroU16", "Integer"),
        ("NonZeroI8", "Integer"),
        ("NonZeroI16", "Integer"),
        ("NonZeroI32", "Integer"),
        ("NonZeroU32", "Integer"),
        ("i64", "BigInt"),
        ("u64", "BigInt"),
        ("isize", "BigInt"),
        ("usize", "BigInt"),
        ("usize", "BigInt"),
        ("NonZeroU64", "BigInt"),
        ("NonZeroU128", "BigInt"),
        ("NonZeroUsize", "BigInt"),
        ("NonZeroI64", "BigInt"),
        ("NonZeroI128", "BigInt"),
        ("NonZeroIsize", "BigInt"),
        ("f32", "Float"),
        ("f64", "Double"),
        ("str", "String"),
        ("String", "String"),
        ("char", "String"),
    ]);
    pub static ref MAPPING_DB_TYPES_TO_RUST: HashMap<&'static str, &'static str> = HashMap::from([
        ("Bool", "bool"),

        ("SmallInt", "i64"),
        ("Integer", "i64"),
        ("BigInt", "i64"),

        ("Float", "f32"),

        ("Double", "f64"),

        ("String", "String"),
    ]);

    pub static ref DEFAULT_TYPE_DB : &'static str = "String";
}
