//Contains public trait
use graph::components::ethereum::EthereumBlock as FullEthereumBlock;
use massbit_common::prelude::anyhow;

pub trait BlockStore: Sync + Send {
    fn get_latest_block_number(&self) -> Option<u64>;
    fn get_earliest_block_number(&self) -> Option<u64>;
    fn store_full_ethereum_blocks(
        &self,
        full_block: &Vec<FullEthereumBlock>,
        network: String,
    ) -> Result<(), anyhow::Error>;
}
