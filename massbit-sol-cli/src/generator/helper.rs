pub fn is_integer_type(data_type: &str) -> bool {
    let vec = vec![
        "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64", "u128", "i128",
    ];
    vec.contains(&data_type)
}
pub fn replace_invalid_identifier_chars(s: &str) -> String {
    s.strip_prefix('$')
        .unwrap_or(s)
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
}
