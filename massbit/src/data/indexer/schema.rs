use rand::rngs::OsRng;
use rand::Rng;
use stable_hash::{SequenceNumber, StableHash, StableHasher};
use std::{fmt, fmt::Display};

use crate::blockchain::Blockchain;
use crate::data::indexer::IndexerManifest;
use crate::prelude::{BlockPtr, CheapClone, DeploymentHash};

#[derive(Debug)]
pub struct IndexerDeploymentEntity {
    pub manifest: IndexerManifestEntity,
    pub failed: bool,
    pub synced: bool,
    pub fatal_error: Option<IndexerError>,
    pub non_fatal_errors: Vec<IndexerError>,
    pub earliest_block: Option<BlockPtr>,
    pub latest_block: Option<BlockPtr>,
    pub reorg_count: i32,
    pub current_reorg_depth: i32,
    pub max_reorg_depth: i32,
}

impl IndexerDeploymentEntity {
    pub fn new(
        source_manifest: &IndexerManifest<impl Blockchain>,
        synced: bool,
        earliest_block: Option<BlockPtr>,
    ) -> Self {
        Self {
            manifest: IndexerManifestEntity::from(source_manifest),
            failed: false,
            synced,
            fatal_error: None,
            non_fatal_errors: vec![],
            earliest_block: earliest_block.cheap_clone(),
            latest_block: earliest_block,
            reorg_count: 0,
            current_reorg_depth: 0,
            max_reorg_depth: 0,
        }
    }
}

#[derive(Debug)]
pub struct IndexerManifestEntity {
    pub spec_version: String,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub schema: String,
    pub features: Vec<String>,
}

impl<'a, C: Blockchain> From<&'a super::IndexerManifest<C>> for IndexerManifestEntity {
    fn from(manifest: &'a super::IndexerManifest<C>) -> Self {
        Self {
            spec_version: manifest.spec_version.to_string(),
            description: manifest.description.clone(),
            repository: manifest.repository.clone(),
            schema: manifest.schema.document.clone().to_string(),
            features: vec![],
        }
    }
}

#[derive(Debug)]
pub struct IndexerError {
    pub indexer_id: DeploymentHash,
    pub message: String,
    pub block_ptr: Option<BlockPtr>,
    pub handler: Option<String>,

    // `true` if we are certain the error is deterministic. If in doubt, this is `false`.
    pub deterministic: bool,
}

impl Display for IndexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.message)?;
        if let Some(handler) = &self.handler {
            write!(f, " in handler `{}`", handler)?;
        }
        if let Some(block_ptr) = &self.block_ptr {
            write!(f, " at block {}", block_ptr)?;
        }
        Ok(())
    }
}

impl StableHash for IndexerError {
    fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
        let IndexerError {
            indexer_id,
            message,
            block_ptr,
            handler,
            deterministic,
        } = self;
        indexer_id.stable_hash(sequence_number.next_child(), state);
        message.stable_hash(sequence_number.next_child(), state);
        block_ptr.stable_hash(sequence_number.next_child(), state);
        handler.stable_hash(sequence_number.next_child(), state);
        deterministic.stable_hash(sequence_number.next_child(), state);
    }
}

pub fn generate_entity_id() -> String {
    // Fast crypto RNG from operating system
    let mut rng = OsRng::new().unwrap();

    // 128 random bits
    let id_bytes: [u8; 16] = rng.gen();

    // 32 hex chars
    // Comparable to uuidv4, but without the hyphens,
    // and without spending bits on a version identifier.
    hex::encode(id_bytes)
}
