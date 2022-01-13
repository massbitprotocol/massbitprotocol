use clap::{App, Arg};
//use logger::core::init_logger;
use massbit_sol::generator::Generator;
use massbit_sol::indexer_deploy::deploy_indexer;
use massbit_sol::indexer_release::release_indexer;
use massbit_sol::parser::SchemaBuilder;
use massbit_sol::INDEXER_ENDPOINT;

fn main() {
    //let res = init_logger(&String::from("massbit-sol-cli"));
    //println!("Log output: {}", res); // Print log output type
    let matches = App::new("massbit-sol")
        .version("1.0")
        .about("Massbit Solana CLI")
        .subcommand(create_gencode_cmd())
        .subcommand(create_deploy_cmd())
        .subcommand(create_genstructure_cmd())
        .subcommand(create_release_cmd())
        .get_matches();
    if let Some(ref matches) = matches.subcommand_matches("gencode") {
        let structure_path = matches.value_of("structure").unwrap_or("instruction.rs");
        let config_path = matches.value_of("config").unwrap_or("config.json");
        let output = matches.value_of("output").unwrap_or("src");
        let generator = Generator::builder()
            .with_structure_path(structure_path)
            .with_config_path(config_path)
            .with_output_dir(output)
            .build();
        let _ = generator.generate();
    } else if let Some(ref matches) = matches.subcommand_matches("deploy") {
        let indexer_url = matches
            .value_of("indexer-url")
            .unwrap_or(INDEXER_ENDPOINT.as_str());
        let project_dir = matches.value_of("project-dir").unwrap_or("./");
        match deploy_indexer(indexer_url, project_dir) {
            Ok(_) => {
                println!("Deploy indexer successfully");
            }
            Err(err) => {
                println!("Error {:?}", &err);
            }
        }
    } else if let Some(ref matches) = matches.subcommand_matches("genstructure") {
        let structure_path = matches.value_of("source").unwrap_or("instruction.rs");
        let name = matches.value_of("name").unwrap_or("instruction");
        let output = matches.value_of("output").unwrap_or("src");
        let enums = matches.values_of("enums").and_then(|values| {
            Some(
                values
                    .map(|value| value.to_string())
                    .collect::<Vec<String>>(),
            )
        });
        let mut schema_builder = SchemaBuilder::builder()
            .with_instruction_path(structure_path)
            .with_output_dir(output)
            .with_enums(enums)
            .with_name(name);
        schema_builder.build()
    } else if let Some(ref matches) = matches.subcommand_matches("release") {
        let project_dir = matches.value_of("project-dir").unwrap_or("./");
        match release_indexer(project_dir) {
            Ok(_) => {
                println!("Create `releases` folder successfully");
            }
            Err(err) => {
                println!("Error {:?}", &err);
            }
        }
    }
}
fn create_gencode_cmd() -> App<'static, 'static> {
    App::new("gencode")
        .about("Generate Rust code & SQL migrations from Instruction structure")
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
}

fn create_deploy_cmd() -> App<'static, 'static> {
    App::new("deploy")
        .about("Deploy compiled indexer binary")
        .arg(
            Arg::with_name("indexer-url")
                .short("u")
                .long("indexer-url")
                .value_name("indexer-url")
                .help("Input indexer entry point")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("project-dir")
                .short("d")
                .long("project-dir")
                .value_name("project-dir")
                .help("Compiled directory")
                .takes_value(true),
        )
}

fn create_release_cmd() -> App<'static, 'static> {
    App::new("release").about("Create release folder").arg(
        Arg::with_name("project-dir")
            .short("d")
            .long("project-dir")
            .value_name("project-dir")
            .help("Compiled directory")
            .takes_value(true),
    )
}

fn create_genstructure_cmd() -> App<'static, 'static> {
    App::new("genstructure")
        .about("Generate Solana smartcontract instruction structure from source code.")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .value_name("source")
                .help("Input instruction source code file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("output")
                .help("Output file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("name")
                .help("Instruction name")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("enums")
                .short("v")
                .long("enums")
                .value_name("enums")
                .help("Enums variants name")
                .multiple(true)
                .takes_value(true),
        )
}
