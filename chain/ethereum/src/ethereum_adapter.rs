use crate::adapter::EthereumAdapter as EthereumAdapterTrait;
use crate::transport::Transport;
use crate::types::LightEthereumBlock;
use ethabi::Token;
use massbit::prelude::web3::types::H256;
use std::collections::HashSet;
use web3::Web3;

use massbit::prelude::*;

pub struct EthereumAdapter {
    url_hostname: Arc<String>,
    web3: Arc<Web3<Transport>>,
}

impl EthereumAdapter {
    pub async fn new(url: &str, transport: Transport) -> Self {
        let hostname = url::Url::parse(url)
            .unwrap()
            .host_str()
            .unwrap()
            .to_string();

        let web3 = Arc::new(Web3::new(transport));

        EthereumAdapter {
            url_hostname: Arc::new(hostname),
            web3,
        }
    }
}

#[async_trait]
impl EthereumAdapterTrait for EthereumAdapter {}
