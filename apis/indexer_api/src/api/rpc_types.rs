// Generic dependencies
use serde::Deserialize;

// The order of params is important to correctly map the API request to this struct
#[derive(Clone, Debug, Deserialize)]
pub struct DeployParams {
    pub config: String,
    pub mapping: String,
    pub schema: String,
    pub subgraph: Option<String>, // .SO doesn't need this parsed config file
}
