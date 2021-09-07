use massbit_runtime_wasm::manifest::{DataSource, DataSourceContext};
use serde_yaml::{self, Value};
use std::fs::File;
use std::io::Read;

const DATASOURCE_PATH: &str = r#"/home/viettai/Massbit/QuickSwap-subgraph/subgraph.yaml"#;
fn main() {
    println!("Datasource test");
    let mut file = File::open(DATASOURCE_PATH).expect("Unable to open file");
    // Refactor: Config to download config file from IPFS instead of just reading from local
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("Unable to read string"); // Get raw query
    let raw_config: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();

    let datasources = DataSource::from_manifest(&raw_config);
    println!("{:?}", datasources);
}
