use clap::{App, Arg};
//use logger::core::init_logger;
use massbit_sol::generator::Generator;
use massbit_sol::indexer_deploy::deploy_indexer;
use massbit_sol::indexer_release::release_indexer;
use massbit_sol::parser::IndexerBuilder;
use massbit_sol::INDEXER_ENDPOINT;

fn main() {
    //let res = init_logger(&String::from("massbit-sol-cli"));
    //println!("Log output: {}", res); // Print log output type
    let matches = App::new("massbit-sol")
        .version("1.0")
        .about("Massbit Solana CLI")
        .subcommand(create_gencode_cmd())
        .subcommand(create_deploy_cmd())
        .subcommand(create_genindexer_cmd())
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
    } else if let Some(ref matches) = matches.subcommand_matches("genindexer") {
        let config_path = matches.value_of("config").unwrap_or("config.json");
        let gen_meta: bool = matches
            .value_of("gen_meta")
            .and_then(|val| val.parse().ok())
            .unwrap_or_default();
        let mut indexer_builder = IndexerBuilder::builder()
            .with_config_path(config_path)
            .gen_meta(gen_meta);
        indexer_builder.build()
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
                .help("compiled directories")
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

fn create_genindexer_cmd() -> App<'static, 'static> {
    App::new("genindexer")
        .about("Generate Solana smartcontract instruction structure from source code.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("config")
                .help("Input indexer configuration file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("gen_inst")
                .short("i")
                .long("gen_inst")
                .value_name("gen_inst")
                .help("Optional generate instruction list")
                .takes_value(true),
        )
}
