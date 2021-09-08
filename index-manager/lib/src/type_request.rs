/**
 *** The objective of this file is to expose types / models from the API request
 *** The IndexConfig will be created with params from this types
 *** After the creation of IndexConfig, this shouldn't be used anymore
 **/
// Generic dependencies
use serde::{Deserialize, Serialize};

// The order of params is important to correctly map the API request to this struct
#[derive(Clone, Debug, Deserialize)]
pub struct DeployParams {
    pub config: String,
    pub mapping: String,
    pub schema: String,
    pub abi: Option<Vec<DeployAbi>>,  // .SO doesn't support uploading ABIs yet, only .WASM need the ABIs
    pub subgraph: Option<String>, // .SO doesn't need this parsed config file
}

// User can upload multiple ABI files. So we need this object to get the abi's name and it's ipfs hash
#[derive(Clone, Debug, Deserialize)]
pub struct DeployAbi {
    pub name: String,
    pub hash: String,
}
