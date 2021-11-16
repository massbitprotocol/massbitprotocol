use clap::{App, Arg};
//use logger::core::init_logger;
use solana_cli::generator::Generator;

fn main() {
    //let res = init_logger(&String::from("solana-cli"));
    //println!("Log output: {}", res); // Print log output type
    let matches = App::new("massbit-sol")
        .version("1.0")
        .about("Massbit Solana CLI")
        .arg(
            Arg::with_name("structure")
                .short("s")
                .long("structure")
                .value_name("structure")
                .help("Input instruction structure file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("config")
                .help("Input config file")
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
    let config_path = matches.value_of("config").unwrap_or("config.json");
    let output = matches.value_of("output").unwrap_or("src");
    let generator = Generator::builder()
        .with_structure_path(structure_path)
        .with_config_path(config_path)
        .with_output_dir(output)
        .build();
    generator.generate();
}
