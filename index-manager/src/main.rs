use std::io;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use jsonrpc_http_server::{
    jsonrpc_core::{self, Compatibility, IoHandler, Params, Value},
    RestApi, Server, ServerBuilder,
};
use serde;
use serde::{Deserialize, Serialize};
use serde_json;
use tokio::runtime::Runtime;

use std::collections::HashMap;
use std::fs::File;

use slog::*;
use anyhow::{anyhow};

#[derive(Clone, Debug, Deserialize)]
struct DeployParams {
    index_name: String,
    config_url: String,
    // start_block
    // abi: String,
}

/// Runtime representation of a data source.
// Note: Not great for memory usage that this needs to be `Clone`, considering how there may be tens
// of thousands of data sources in memory at once.
// #[derive(Clone, Debug)]
// pub struct DataSource {
//     pub kind: String,
//     pub network: Option<String>,
//     pub name: String,
//     pub source: Source,
//     pub mapping: Mapping,
//     pub context: Arc<Option<DataSourceContext>>,
//     pub creation_block: Option<BlockNumber>,
//     pub contract_abi: Arc<MappingABI>,
// }

#[tokio::main]
async fn main() {
    let server = JsonRpcServer::serve(
        "127.0.0.1:3030".to_string(),
    );
    server.wait();
}

fn deploy_handler(
    params: DeployParams,
) {
    loop {
        IndexDeployment::load_file(params.config_url.clone()); // We are using loop for the indexing demo, but using clone is not efficient here
        thread::sleep(Duration::from_secs(1));
    }
}

// fn from_manifest(
//     kind: String,
//     network: Option<String>,
//     name: String,
//     source: Source,
//     mapping: Mapping,
//     context: Option<DataSourceContext>,
// ) -> Result<Self, Error> {
//     // Data sources in the manifest are created "before genesis" so they have no creation block.
//     let creation_block = None;
//     let contract_abi = mapping
//         .find_abi(&source.abi)
//         .with_context(|| format!("data source `{}`", name))?;
//
//     Ok(DataSource {
//         kind,
//         network,
//         name,
//         source,
//         mapping,
//         context: Arc::new(context),
//         creation_block,
//         contract_abi,
//     })
// }

//
// Test
//
#[derive(Default)]
struct TextResolver {
    texts: HashMap<String, String>,
}
impl TextResolver {
    fn add(&mut self, link: &str, text: &str) {
        self.texts.insert(link.to_owned(), text.to_owned());
    }
}
const GQL_SCHEMA: &str = "type Thing @entity { id: ID! }";
const ABI: &str = "[{\"type\":\"function\", \"inputs\": [{\"name\": \"i\",\"type\": \"uint256\"}],\"name\":\"get\",\"outputs\": [{\"type\": \"address\",\"name\": \"o\"}]}]";
const MAPPING: &str = "export function handleGet(call: getCall): void {}";
// async fn resolve_manifest(text: &str) -> SubgraphManifest {
async fn resolve_manifest(text: &str) {
    let mut resolver = TextResolver::default();
    // let id = DeploymentHash::new("Qmmanifest").unwrap();
    // resolver.add("a".as_str(), text);
    resolver.add("/ipfs/Qmschema", GQL_SCHEMA);
    resolver.add("/ipfs/Qmabi", ABI);
    resolver.add("/ipfs/Qmmapping", MAPPING);
    // SubgraphManifest::resolve(id, &resolver, &LOGGER)
    //     .await
    //     .expect("Parsing simple manifest works")
}

//
// JSON RPC HTTP SERVER. TODO: migrate to a separate cargo
//
pub struct JsonRpcServer {
    http_addr: String,
}

impl JsonRpcServer {
    fn serve(
        http_addr: String,
    ) -> jsonrpc_http_server::Server {
        let mut handler = IoHandler::with_compatibility(Compatibility::Both);

        // If we want to use tokio::spawn, need to grab the hackie code from the graph that resolve running tokio spawn with json_rpc_http_server
        // Reason: https://stackoverflow.com/questions/61292425/how-to-run-an-asynchronous-task-from-a-non-main-thread-in-tokio
        handler.add_method("index_deploy", |params: Params| {
            thread::spawn(|| {
                let params: DeployParams = params.parse().unwrap(); // Refactor to add param check
                println!("Received an index request from {}", params.index_name); // Refactor to use slog logger

                deploy_handler(params);
            });
            Ok(Value::String("Index deployed success".into()))
        });

        let server = ServerBuilder::new(handler)
            .start_http(&http_addr.parse().unwrap())
            .expect("Unable to start RPC server");

        server
    }
}

//
// Index Deployment
//
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseIndexDeployment {
    // pub id: DeploymentHash,
    pub spec_version: String,
    // #[serde(default)]
    // pub features: BTreeSet<SubgraphFeature>,
    // pub description: Option<String>,
    // pub repository: Option<String>,
    // pub schema: S,
    // pub data_sources: Vec<D>,
    // pub graft: Option<Graft>,
    // #[serde(default)]
    // pub templates: Vec<T>,
    // #[serde(skip_serializing, default)]
    // pub chain: PhantomData<C>,
}

pub type IndexDeployment = BaseIndexDeployment; // This is refactored from BaseSubgraphManifest. TODO: Check why this needs a base struct
impl IndexDeployment {
    /// Entry point for resolving a subgraph definition.
    /// Right now the only supported links are of the form:
    /// `/ipfs/QmUmg7BZC1YP1ca66rRtWKxpXp77WgVHrnv263JtDuvs2k`
    // pub async fn resolve(
    pub fn resolve(
        // id: DeploymentHash,
        // resolver: &impl LinkResolver,
        logger: &Logger,
    // ) -> Result<Self, SubgraphManifestResolveError> {
    ) {
        // let link = Link {
        //     link: id.to_string(),
        // };
        // info!(logger, "Resolve manifest"; "link" => &link.link);
        let logger = logger.new(o!("component" => "BlockWriter"));

        // let file_bytes = resolver;
            // .cat(logger, &link)
            // .await
            // .map_err(SubgraphManifestResolveError::ResolveError)?;

        // let file = String::from_utf8(file_bytes.to_vec());
            // .map_err(|_| SubgraphManifestResolveError::NonUtf8)?;
        // let raw: serde_yaml::Value = serde_yaml::from_str(&file)?;
        //
        // let raw_mapping = match raw {
        //     serde_yaml::Value::Mapping(m) => m,
        //     _ => return Err(SubgraphManifestResolveError::InvalidFormat),
        // };
        //
        // Self::resolve_from_raw(id, raw_mapping, resolver, logger).await
    }

    // pub async fn resolve_from_raw(
    //     id: DeploymentHash,
    //     mut raw: serde_yaml::Mapping,
    //     resolver: &impl LinkResolver,
    //     logger: &Logger,
    // ) -> Result<Self, SubgraphManifestResolveError> {
    //     // Inject the IPFS hash as the ID of the subgraph into the definition.
    //     raw.insert(
    //         serde_yaml::Value::from("id"),
    //         serde_yaml::Value::from(id.to_string()),
    //     );
    //
    //     // Parse the YAML data into an UnresolvedSubgraphManifest
    //     let unresolved: UnresolvedSubgraphManifest<C> = serde_yaml::from_value(raw.into())?;
    //
    //     debug!(logger, "Features {:?}", unresolved.features);
    //
    //     unresolved
    //         .resolve(&*resolver, logger)
    //         .await
    //         .map_err(SubgraphManifestResolveError::ResolveError)
    // }

    // Hughie: Lazily read config from a local file
    fn load_file(
        config_url: String,
    ) {
        let f = File::open(config_url).unwrap();
        let data: serde_yaml::Value = serde_yaml::from_reader(f).unwrap();

        let schemaFile = data["schema"]["file"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(anyhow!("Could not find schema file"));
        println!("Schema: {}",schemaFile.unwrap()); // Refactor to use slog logge

        let kind = data["dataSources"][0]["kind"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(anyhow!("Could not find network kind"));
        println!("Kind: {}",kind.unwrap()); // Refactor to use slog logge
    }
}