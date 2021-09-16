use ethabi::Token;
use graph::util::futures::retry;
use massbit::components::store::EthereumCallCache;
use massbit::prelude::web3::types::H256;
use std::collections::HashSet;
use web3::Web3;

use crate::adapter::{
    EthereumAdapter as EthereumAdapterTrait, EthereumContractCall, EthereumContractCallError,
};
use crate::transport::Transport;
use crate::types::LightEthereumBlock;

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
impl EthereumAdapterTrait for EthereumAdapter {
    fn latest_block(
        &self,
    ) -> Box<dyn Future<Item = LightEthereumBlock, Error = Error> + Send + Unpin> {
        todo!()
    }

    fn load_blocks(
        &self,
        block_hashes: HashSet<H256>,
    ) -> Box<dyn Stream<Item = LightEthereumBlock, Error = Error> + Send> {
        todo!()
    }

    fn contract_call(
        &self,
        call: EthereumContractCall,
        cache: Arc<dyn EthereumCallCache>,
    ) -> Box<dyn Future<Item = Vec<Token>, Error = EthereumContractCallError> + Send> {
        todo!()
    }
}
