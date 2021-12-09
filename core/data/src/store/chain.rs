use massbit_common::cheap_clone::CheapClone;
use massbit_common::prelude::anyhow;
use massbit_common::prelude::stable_hash::utils::AsBytes;
use massbit_common::prelude::stable_hash::{SequenceNumber, StableHash, StableHasher};
use std::convert::TryFrom;
use std::fmt;
use std::fmt::Write;

pub type BlockNumber = i64;
pub const BLOCK_NUMBER_MAX: BlockNumber = i64::MAX;

/// A simple marker for byte arrays that are really block hashes
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockHash(pub Box<[u8]>);

impl BlockHash {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "0x{}", hex::encode(&self.0))
    }
}

impl CheapClone for BlockHash {}

impl From<Vec<u8>> for BlockHash {
    fn from(bytes: Vec<u8>) -> Self {
        BlockHash(bytes.as_slice().into())
    }
}

/// A block hash and block number from a specific Ethereum block.
///
/// Block numbers are signed 32 bit integers
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockPtr {
    pub hash: BlockHash,
    pub number: BlockNumber,
}

impl CheapClone for BlockPtr {}

impl StableHash for BlockPtr {
    fn stable_hash<H: StableHasher>(&self, mut sequence_number: H::Seq, state: &mut H) {
        AsBytes(self.hash.0.as_ref()).stable_hash(sequence_number.next_child(), state);
        self.number.stable_hash(sequence_number.next_child(), state);
    }
}

impl BlockPtr {
    /// Encodes the block hash into a hexadecimal string **without** a "0x" prefix.
    /// Hashes are stored in the database in this format.
    pub fn hash_hex(&self) -> String {
        let mut s = String::with_capacity(self.hash.0.len() * 2);
        for b in self.hash.0.iter() {
            write!(s, "{:02x}", b).unwrap();
        }
        s
    }

    /// Block number to be passed into the store. Panics if it does not fit in an i32.
    pub fn block_number(&self) -> BlockNumber {
        self.number
    }

    pub fn hash_slice(&self) -> &[u8] {
        self.hash.0.as_ref()
    }
}

impl fmt::Display for BlockPtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{} ({})", self.number, self.hash_hex())
    }
}

impl From<(Vec<u8>, i64)> for BlockPtr {
    fn from((bytes, number): (Vec<u8>, i64)) -> Self {
        BlockPtr {
            hash: BlockHash::from(bytes),
            number,
        }
    }
}

impl TryFrom<(&str, i64)> for BlockPtr {
    type Error = anyhow::Error;

    fn try_from((hash, number): (&str, i64)) -> Result<Self, Self::Error> {
        let hash = hash.trim_start_matches("0x");
        Ok(BlockPtr::from((hash.as_bytes().to_vec(), number)))
    }
}

impl TryFrom<(&[u8], i64)> for BlockPtr {
    type Error = anyhow::Error;

    fn try_from((bytes, number): (&[u8], i64)) -> Result<Self, Self::Error> {
        Ok(BlockPtr::from((bytes.to_vec(), number)))
    }
}

impl From<BlockPtr> for BlockNumber {
    fn from(ptr: BlockPtr) -> Self {
        ptr.number
    }
}
