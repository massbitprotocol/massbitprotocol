use crate::orm::models::Indexer;
use chain_solana::SolanaIndexerManifest;
use massbit_common::prelude::anyhow;
use std::path::PathBuf;

#[derive(Default)]
pub struct IndexerManager {}

impl IndexerManager {
    pub async fn init_indexer(
        &mut self,
        indexer: &Indexer,
        manifest: &SolanaIndexerManifest,
        mapping_path: &PathBuf,
        graphql: &PathBuf,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
