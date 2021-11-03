use schemars::{schema_for, JsonSchema};
use serde_json::{from_str, to_string_pretty};
use solana_cli::test_origin::{
    InitializeMarketInstruction, MarketInstruction, NewOrderInstructionV3, OrderType,
};
use std::fs;

fn main() {
    let schema = schema_for!(MarketInstruction);
    let data = serde_json::to_string_pretty(&schema).unwrap();
    println!("{}", &data);
    fs::write("solana-cli/src/test_origin.json", data).expect("Unable to write file");

    let schema_path = "src/test_origin.json";
    schemafy_lib::Generator::builder()
        .with_root_name_str("MarketInstruction")
        .with_input_file(schema_path)
        .build()
        .generate_to_file("solana-cli/src/test_new.rs")
        .unwrap();
}
