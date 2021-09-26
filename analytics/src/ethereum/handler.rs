//Public trait for ethereum metric
use massbit_common::prelude::anyhow;
use massbit_chain_ethereum::data_type::ExtBlock;

pub trait EthereumHandler : Sync + Send {
    fn handle_block(&self, &ExtBlock) -> Result<(), anyhow::Error>;
    fn handle_blocks(&self, &Vec<ExtBlock>) -> Result<(), anyhow::Error>;
    fn store_full_ethereum_blocks(&self, full_block: &Vec<FullEthereumBlock>) -> Result<(), anyhow::Error>;
}
