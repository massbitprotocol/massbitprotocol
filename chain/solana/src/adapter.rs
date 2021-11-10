use crate::chain::Chain;
use crate::data_source::DataSource;
use massbit::blockchain as bc;
use massbit::blockchain::HostFn;
use massbit::prelude::*;

#[derive(Clone, Debug)]
pub struct SolanaAdapter {}

#[derive(Clone)]
pub struct SolanaNetworkAdapter {
    pub adapter: Arc<SolanaAdapter>,
}
#[derive(Clone)]
pub struct SolanaNetworkAdapters {
    pub adapters: Vec<SolanaNetworkAdapter>,
}

pub struct RuntimeAdapter {
    pub(crate) sol_adapters: Arc<SolanaNetworkAdapters>,
}
impl bc::RuntimeAdapter<Chain> for RuntimeAdapter {
    fn host_fns(&self, ds: &DataSource) -> Result<Vec<HostFn>, Error> {
        todo!()
    }
}
