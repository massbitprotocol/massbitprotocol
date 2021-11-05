use schemars::{schema_for, JsonSchema};
use serde_json::{from_str, to_string_pretty};
//use solana_cli::jsonschema::generator::Generator;
use solana_cli::instruction::generator::Generator;
use solana_cli::test_origin::{
    InitializeMarketInstruction, MarketInstruction, NewOrderInstructionV3, OrderType,
};
use std::fs;
fn main() {
    let schema = schema_for!(MarketInstruction);
    let data = serde_json::to_string_pretty(&schema).unwrap();
    //println!("{}", &data);
    fs::write("solana-cli/src/test_origin.json", data).expect("Unable to write file");

    let schema_path = "src/serum_instruction.json";
    Generator::builder()
        .with_root_name_str("MarketInstruction")
        .with_input_file(schema_path)
        .build()
        .generate_to_file("solana-cli/src/serum.rs")
        .unwrap();

    // Create instruction
    let market_intruction = MarketInstruction::CloseOpenOrders;
    let pack_mi = market_intruction.pack();
    let unpack_mi = MarketInstruction::unpack(pack_mi.as_slice());
    // println!(
    //     "market_intruction: {:?}, endcode: {:?}, decode: {:?}",
    //     market_intruction, pack_mi, unpack_mi
    // );
}
