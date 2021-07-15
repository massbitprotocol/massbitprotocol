use node_template_runtime::{Block, Header, Hash, DigestItem};
use std::str::FromStr;
use sp_runtime::Digest;

// Massbit Dependencies
use massbit_chain_substrate::data_type::SubstrateBlock;

// pub fn generate_substrate_block() -> SubstrateBlock{
//     let block: SubstrateBlock = Block {
//         header: Header {
//             parent_hash: Hash::from_str("0x5611f005b55ffb1711eaf3b2f5557c788aa2e3d61b1a833f310f9c7e12a914f7").unwrap(),
//             number: 610,
//             state_root: Hash::from_str("0x173717683ea4459d15d532264aa7c51657cd65d204c033834ffa62f9ea69e78b").unwrap(),
//             extrinsics_root: Hash::from_str("0x732ea723e3ff97289d22f2a4a52887329cd37c3b694a4d563979656d1aa6b7ee").unwrap(),
//             digest: Digest {
//                 logs: [
//                     DigestItem::ChangesTrieRoot(Hash::from_str("0x173717683ea4459d15d532264aa7c51657cd65d204c033834ffa62f9ea69e78b").unwrap()),
//                 ].to_vec()
//             }
//         },
//         extrinsics: [].to_vec()
//     };
//     block
// }