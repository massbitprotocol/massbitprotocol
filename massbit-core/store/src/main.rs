use node_template_runtime;
use std::{path::PathBuf};
use node_template_runtime::{Header, Block, DigestItem, Hash};
use std::str::FromStr;

// Massbit dependencies
use plugin::manager::PluginManager;
use massbit_chain_substrate::data_type::SubstrateBlock;
use sp_runtime::Digest;

fn main() {
    // This object will later be used to write test
    let block: SubstrateBlock = Block {
        header: Header {
            parent_hash: Hash::from_str("0x5611f005b55ffb1711eaf3b2f5557c788aa2e3d61b1a833f310f9c7e12a914f7").unwrap(),
            number: 610,
            state_root: Hash::from_str("0x173717683ea4459d15d532264aa7c51657cd65d204c033834ffa62f9ea69e78b").unwrap(),
            extrinsics_root: Hash::from_str("0x732ea723e3ff97289d22f2a4a52887329cd37c3b694a4d563979656d1aa6b7ee").unwrap(),
            digest: Digest {
                logs: [
                    DigestItem::ChangesTrieRoot(Hash::from_str("0x173717683ea4459d15d532264aa7c51657cd65d204c033834ffa62f9ea69e78b").unwrap()),
                ].to_vec()
            }
        },
        extrinsics: [].to_vec()
    };

    let library_path = PathBuf::from("./target/release/libblock.so".to_string());
    let mut plugins = PluginManager::new();
    unsafe {
        plugins
            .load(&library_path)
            .expect("plugin loading failed");
    }

    plugins.handle_block(&String::from("postgres://graph-node:let-me-in@localhost"), &block);
}
