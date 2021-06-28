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
    let mut handler = IoHandler::default();

    // If we want to use tokio::spawn, need to grab the hackie code from the graph that resolve running tokio spawn with json_rpc_http_server
    // Reason: https://stackoverflow.com/questions/61292425/how-to-run-an-asynchronous-task-from-a-non-main-thread-in-tokio
    handler.add_method("index_deploy", |params: Params| {
        thread::spawn(|| {
            println!("Received an index request"); // Refactor to use logger
            let params = params.parse().unwrap();

            // Add param check

            deploy_handler(params);
        });
        Ok(Value::String("hello".into()))
    });

    let server = ServerBuilder::new(handler)
        .start_http(&"127.0.0.1:3030".parse().unwrap())
        .expect("Unable to start RPC server");
    server.wait();


    // TODO: Refactor with JsonRpcServer later
    // let arc_self = Arc::new(JsonRpcServer {
    //     http_addr,
    // });
    // JsonRpcServer::serve(
    //     "127.0.0.1:3030".to_string(),
    // );
}

fn deploy_handler(
    params: DeployParams,
) {
    println!("{}",params.index_name);

    // Load manifest from IPFS

    // Read manifest
    const YAML: &str = "
        dataSources: []
        schema:
          file:
            /: /ipfs/Qmschema
        specVersion: 0.0.2
       ";

    // let manifest = resolve_manifest(YAML).await;
    // SubgraphManifest::resolve(id, &resolver, &LOGGER)
    SubgraphManifest::load_file(params.config_url);
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


// TODO: Refactor with JsonRpcServer later
// pub struct JsonRpcServer {
//     http_addr: String,
// }
//
// impl JsonRpcServer {
//     fn serve(
//         http_addr: String,
//     ) -> Result<CustomServer, io::Error> {
//         let server = ServerBuilder::new(handler)
//             // .cors(DomainsValidation::AllowOnly(vec![AccessControlAllowOrigin::Null]))
//             .start_http(&http_addr.parse().unwrap())
//             .expect("Unable to start RPC server");
//         server
//     }
//
//     /// Handler for the `subgraph_deploy` endpoint.
//     async fn deploy_handler(
//         &self,
//         // params: SubgraphDeployParams,
//     ) -> Result<Value, jsonrpc_core::Error> {
//         Ok(serde_json::to_value("A").expect("invalid deploy"))
//     }
// }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseSubgraphManifest {
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

pub type SubgraphManifest = BaseSubgraphManifest;


impl SubgraphManifest {
    /// Entry point for resolving a subgraph definition.
    /// Right now the only supported links are of the form:
    /// `/ipfs/QmUmg7BZC1YP1ca66rRtWKxpXp77WgVHrnv263JtDuvs2k`
    // pub async fn resolve(
    pub fn resolve(
        // id: DeploymentHash,
        // resolver: &impl LinkResolver,
        // logger: &Logger,
    // ) -> Result<Self, SubgraphManifestResolveError> {
    ) {
        // let link = Link {
        //     link: id.to_string(),
        // };
        // info!(logger, "Resolve manifest"; "link" => &link.link);

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

    // Hughie: New introduce
    fn load_file(
        config_url: String,
    ) {
        let f = File::open(config_url).unwrap();
        let data: serde_yaml::Value = serde_yaml::from_reader(f).unwrap();
        let schemaFile = data["schema"]["file"]
            .as_str()
            .map(|s| s.to_string()).unwrap();
            // .ok_or(anyhow!("Could not find key foo.bar in something.yaml"));
        println!("Schema: {}",schemaFile);

        let kind = data["dataSources"][0]["kind"]
            .as_str()
            .map(|s| s.to_string()).unwrap();
        // .ok_or(anyhow!("Could not find key foo.bar in something.yaml"));
        println!("Kind: {}",kind);
    }
}