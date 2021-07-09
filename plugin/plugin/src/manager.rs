use crate::core::{BlockHandler, PluginDeclaration, PluginRegistrar as PluginRegistrarTrait};
use libloading::Library;
use massbit_chain_substrate::data_type::SubstrateBlock;
use std::{alloc::System, collections::HashMap, error::Error, ffi::OsStr, rc::Rc};
use store::Store;

#[global_allocator]
static ALLOCATOR: System = System;

pub struct PluginManager<'a> {
    store: &'a dyn Store,
    libraries: Vec<Rc<Library>>,
    block_handlers: HashMap<String, BlockHandlerProxy>,
}

impl<'a> PluginManager<'a> {
    pub fn new(store: &dyn Store) -> PluginManager {
        PluginManager {
            store,
            libraries: vec![],
            block_handlers: HashMap::default(),
        }
    }

    pub unsafe fn load<P: AsRef<OsStr>>(&mut self, library_path: P) -> Result<(), Box<dyn Error>> {
        let library = Rc::new(Library::new(library_path)?);
        library
            .get::<*mut Option<&dyn Store>>(b"STORE\0")?
            .write(Some(self.store));

        let plugin_decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();
        let mut registrar = PluginRegistrar::new(Rc::clone(&library));
        (plugin_decl.register)(&mut registrar);

        self.block_handlers.extend(registrar.block_handlers);
        self.libraries.push(library);
        Ok(())
    }

    pub fn handle_block(
        &self,
        block_handler: &str,
        block: &SubstrateBlock,
    ) -> Result<(), Box<dyn Error>> {
        self.block_handlers
            .get(block_handler)
            .ok_or_else(|| format!("\"{}\" not found", block_handler))?
            .handle_block(block)
    }
}

struct PluginRegistrar {
    lib: Rc<Library>,
    block_handlers: HashMap<String, BlockHandlerProxy>,
}

impl PluginRegistrar {
    fn new(lib: Rc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            lib,
            block_handlers: HashMap::default(),
        }
    }
}

impl PluginRegistrarTrait for PluginRegistrar {
    fn register_block_handler(&mut self, name: &str, function: Box<dyn BlockHandler>) {
        let proxy = BlockHandlerProxy {
            function,
            _lib: Rc::clone(&self.lib),
        };
        self.block_handlers.insert(name.to_string(), proxy);
    }
}

pub struct BlockHandlerProxy {
    function: Box<dyn BlockHandler>,
    _lib: Rc<Library>,
}

impl BlockHandler for BlockHandlerProxy {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn std::error::Error>> {
        self.function.handle_block(block)
    }
}
