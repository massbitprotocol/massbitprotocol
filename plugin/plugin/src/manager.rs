use crate::core::{BlockHandler, PluginDeclaration, PluginRegistrar as PluginRegistrarTrait};
use libloading::Library;
use massbit_chain_substrate::data_type::SubstrateBlock;
use std::{alloc::System, collections::HashMap, error::Error, ffi::OsStr, rc::Rc};
use store::Store;

#[global_allocator]
static ALLOCATOR: System = System;

pub struct PluginManager<'a> {
    store: &'a dyn Store,
    plugins: Vec<Rc<Library>>,
    substrate_block_handlers: HashMap<String, HashMap<String, SubstrateBlockHandlerProxy>>,
}

impl<'a> PluginManager<'a> {
    pub fn new(store: &dyn Store) -> PluginManager {
        PluginManager {
            store,
            plugins: vec![],
            substrate_block_handlers: HashMap::default(),
        }
    }

    pub unsafe fn load<P: AsRef<OsStr>>(
        &mut self,
        plugin_id: &str,
        plugin_path: P,
    ) -> Result<(), Box<dyn Error>> {
        let plugin = Rc::new(Library::new(plugin_path)?);
        // inject store to plugin
        plugin
            .get::<*mut Option<&dyn Store>>(b"STORE\0")?
            .write(Some(self.store));

        let plugin_decl = plugin
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();
        let mut registrar = PluginRegistrar::new(Rc::clone(&plugin));
        (plugin_decl.register)(&mut registrar);

        self.substrate_block_handlers
            .entry(plugin_id.to_string())
            .or_insert(HashMap::default())
            .extend(registrar.substrate_block_handlers);
        self.plugins.push(plugin);

        Ok(())
    }

    pub fn handle_substrate_block(
        &self,
        plugin_id: &str,
        handler: &str,
        block: &SubstrateBlock,
    ) -> Result<(), Box<dyn Error>> {
        self.substrate_block_handlers
            .get(plugin_id)
            .ok_or_else(|| format!("\"{}\" not found", plugin_id))?
            .get(handler)
            .ok_or_else(|| format!("\"{}\" not found", handler))?
            .handle_substrate_block(block)
    }
}

struct PluginRegistrar {
    lib: Rc<Library>,
    substrate_block_handlers: HashMap<String, SubstrateBlockHandlerProxy>,
}

impl PluginRegistrar {
    fn new(lib: Rc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            lib,
            substrate_block_handlers: HashMap::default(),
        }
    }
}

impl PluginRegistrarTrait for PluginRegistrar {
    fn register_block_handler(&mut self, name: &str, handler: Box<dyn BlockHandler>) {
        let proxy = SubstrateBlockHandlerProxy {
            handler,
            _lib: Rc::clone(&self.lib),
        };
        self.substrate_block_handlers
            .insert(name.to_string(), proxy);
    }
}

/// A proxy object which wraps a [`BlockHandler`] and makes sure it can't outlive
/// the library it came from.
pub struct SubstrateBlockHandlerProxy {
    handler: Box<dyn BlockHandler>,
    _lib: Rc<Library>,
}

impl BlockHandler for SubstrateBlockHandlerProxy {
    fn handle_substrate_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn Error>> {
        self.handler.handle_substrate_block(block)
    }
}
