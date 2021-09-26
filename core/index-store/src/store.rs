//Contains public trait
use graph::components::ethereum::EthereumBlock as FullEthereumBlock;
use massbit_common::prelude::anyhow;

pub trait BlockStore : Sync + Send {
    fn store_full_ethereum_blocks(&self, full_block: &Vec<FullEthereumBlock>) -> Result<(), anyhow::Error>;
}
