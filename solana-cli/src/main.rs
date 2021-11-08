use schemars::{schema_for, JsonSchema};
use serde_json::{from_str, to_string_pretty};
//use solana_cli::jsonschema::generator::Generator;
use clap::{App, Arg};
use solana_cli::generator::Generator;
use solana_cli::test_origin::{
    InitializeMarketInstruction, MarketInstruction, NewOrderInstructionV3, OrderType,
};
use std::fs;

//const OUTPUT_PATH = "../solana-cli/src/serum.rs";
const OUTPUT_PATH: &str = "code-compiler/generated/serum-index/src/generated/instruction.rs";
fn main() {
    let matches = App::new("massbit-sol")
        .version("1.0")
        .about("Massbit Solana CLI")
        .arg(
            Arg::with_name("structure")
                .short("s")
                .long("structure")
                .value_name("structure")
                .help("Input instruction structure")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("output")
                .help("Output directory")
                .takes_value(true),
        )
        .get_matches();
    let structure_path = matches.value_of("structure").unwrap_or("instruction.json");
    let output = matches.value_of("output").unwrap_or("src/generated");
    let generator = Generator::builder()
        .with_structure_path(structure_path)
        .with_output_dir(output)
        .build();
    generator.generate();
}
/*
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
        .generate_to_file(OUTPUT_PATH)
        .unwrap();

    Create instruction
    let market_intruction = MarketInstruction::CloseOpenOrders;
    let pack_mi = market_intruction.pack();
    let unpack_mi = MarketInstruction::unpack(pack_mi.as_slice());
    println!(
        "market_intruction: {:?}, endcode: {:?}, decode: {:?}",
        market_intruction, pack_mi, unpack_mi
    );
}
*/
