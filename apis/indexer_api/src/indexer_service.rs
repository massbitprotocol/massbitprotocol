use crate::api::rpc_types::DeployParams;
use futures::channel::mpsc;
use futures::channel::mpsc::{Receiver, Sender};
use futures::future::FutureExt;
use futures::{StreamExt, TryFutureExt};
use jsonrpc_core::{Error, Params, Result as JsonRpcResult};
use massbit::prelude::{
    serde::Serialize,
    serde_json::{self, json, Value},
};
use tokio;
use tokio02_spawn::core::tokio02_spawn;

pub struct IndexerService {
    pub task_sender: Sender<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>,
    pub task_receiver: Receiver<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>,
}

impl IndexerService {
    pub fn new() -> Self {
        let (task_sender, task_receiver) =
            mpsc::channel::<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>(100);

        IndexerService {
            task_sender,
            task_receiver,
        }
    }
}

impl IndexerService {
    pub fn deploy(&self, params: DeployParams) -> JsonRpcResult<Value> {
        Box::pin(tokio02_spawn(
            self.task_sender.clone(),
            async move { deploy_handler(params).await }.boxed(),
        ))
        .compat();
        Ok(json!({
            "code": 0,
            "message": "Indexer deploying"
        }))
    }
}

async fn deploy_handler(params: DeployParams) -> JsonRpcResult<Value> {
    // let index_config = IndexConfigIpfsBuilder::default()
    //     .config(&params.config)
    //     .await
    //     .mapping(&params.mapping)
    //     .await
    //     .schema(&params.schema)
    //     .await
    //     //.abi(params.abi)
    //     //.await
    //     .subgraph(&params.subgraph)
    //     .await
    //     .build();
    // // Set up logger
    // let logger = logger(false);
    // let ipfs_addresses = vec![IPFS_ADDRESS.to_string()];
    // let ipfs_clients: Vec<IpfsClient> = create_ipfs_clients(&ipfs_addresses).await;
    //
    // // Convert the clients into a link resolver. Since we want to get past
    // // possible temporary DNS failures, make the resolver retry
    // let link_resolver = Arc::new(LinkResolver::from(ipfs_clients));
    // // Create a component and indexer logger factory
    // let logger_factory = LoggerFactory::new(logger.clone());
    // let deployment_hash = DeploymentHash::new(index_config.identifier.hash.clone())?;
    // let logger = logger_factory.indexer_logger(&DeploymentLocator::new(
    //     DeploymentId(0),
    //     deployment_hash.clone(),
    // ));
    // info!("Ipfs {:?}", &deployment_hash.to_ipfs_link());
    // // let raw: serde_yaml::Mapping = {
    // //     let file_bytes = link_resolver
    // //         .cat(&logger, &deployment_hash.to_ipfs_link())
    // //         .await
    // //         .map_err(|e| {
    // //             error!("{:?}", &e);
    // //             IndexerRegistrarError::ResolveError(IndexerManifestResolveError::ResolveError(e))
    // //         })?;
    // //
    // //     serde_yaml::from_slice(&file_bytes)
    // //         .map_err(|e| IndexerRegistrarError::ResolveError(e.into()))?
    // // };
    // // TODO: Maybe break this into two different struct (So and Wasm) so we don't have to use Option
    // // let mut manifest = IndexerManifest::<Chain>::resolve_from_raw(
    // //     &logger,
    // //     deployment_hash.cheap_clone(),
    // //     raw,
    // //     // Allow for infinite retries for indexer definition files.
    // //     &link_resolver.as_ref().clone().with_retries(),
    // //     MAX_SPEC_VERSION.clone(),
    // // )
    // // .await
    // // .context("Failed to resolve indexer from IPFS")?;
    // let manifest: Option<IndexerManifest<Chain>> = match &params.subgraph {
    //     Some(v) => Some(
    //         get_indexer_manifest(DeploymentHash::new(v)?, link_resolver)
    //             .await
    //             .unwrap(),
    //     ),
    //     None => {
    //         println!(".SO mapping doesn't have parsed data source");
    //         //vec![]
    //         None
    //     }
    // };
    // // Create tables for the new index and track them in hasura
    // //run_ddl_gen(&index_config).await;
    //
    // // Create a new indexer so we can keep track of it's status
    // //IndexStore::insert_new_indexer(&index_config);
    // let config_value = read_config_file(&index_config.config);
    // let network = config_value["dataSources"][0]["kind"].as_str().unwrap();
    // let name = config_value["dataSources"][0]["name"].as_str().unwrap();
    // IndexerStore::create_indexer(
    //     index_config.identifier.hash.clone(),
    //     String::from(name),
    //     String::from(network),
    //     &params.subgraph,
    // );
    // Start the adapter for the index
    //adapter_init(&index_config, &manifest).await?;
    Ok(serde_json::to_value("Deploy index success").expect("Unable to deploy new index"))
}
