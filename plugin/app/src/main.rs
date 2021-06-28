use plugin_manager::PluginManager;
use std::{env, path::PathBuf};
use types::{SubstrateBlock, SubstrateExtrinsic};

fn main() {
    let mut args = env::args().skip(1);
    let library_path = PathBuf::from(args.next().unwrap());

    let mut plugins = PluginManager::new();
    unsafe {
        plugins
            .load(&library_path)
            .expect("Function loading failed");
    }

    let block = SubstrateBlock { idx: 1 };
    plugins.handle_block(&block);

    let extrinsic = SubstrateExtrinsic { idx: 1 };
    plugins.handle_extrinsic(&extrinsic);
}
