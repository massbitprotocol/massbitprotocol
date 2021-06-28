#[macro_use]
extern crate diesel_derive_table;
#[macro_use]
extern crate diesel;

use diesel::pg::Pg;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use libloading::Library;
use libloading::Symbol;
use plugin_core::{
    BlockHandler, EventHandler, ExtrinsicHandler, InvocationError, PluginDeclaration,
};
use std::{alloc::System, collections::HashMap, env, ffi::OsStr, io, path::PathBuf, rc::Rc};
use types::{SubstrateBlock, SubstrateEvent, SubstrateExtrinsic};

#[global_allocator]
static ALLOCATOR: System = System;

#[derive(Default)]
pub struct PluginManager {
    block_handlers: HashMap<String, BlockHandlerProxy>,
    extrinsic_handlers: HashMap<String, ExtrinsicHandlerProxy>,
    event_handlers: HashMap<String, EventHandlerProxy>,
    libraries: Vec<Rc<Library>>,
}

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager::default()
    }

    pub unsafe fn load<P: AsRef<OsStr>>(&mut self, library_path: P) -> io::Result<()> {
        let library = Rc::new(Library::new(library_path)?);
        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();
        let mut registrar = PluginRegistrar::new(Rc::clone(&library));
        (decl.register)(&mut registrar);

        let conn: Symbol<*mut Option<PgConnection>> = library.get(b"CONN\0").unwrap();
        let database_url = "postgres://postgres:postgres@localhost".to_string();
        let _conn = PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
        **conn = Some(_conn);

        self.block_handlers.extend(registrar.block_handlers);
        self.extrinsic_handlers.extend(registrar.extrinsic_handlers);
        self.event_handlers.extend(registrar.event_handlers);
        self.libraries.push(library);
        Ok(())
    }

    pub fn handle_block(&self, block: &SubstrateBlock) {
        for (_, handler) in &self.block_handlers {
            let result = handler.handle_block(block);
        }
    }

    pub fn handle_extrinsic(&self, extrinsic: &SubstrateExtrinsic) {
        for (_, handler) in &self.extrinsic_handlers {
            let result = handler.handle_extrinsic(extrinsic);
        }
    }

    pub fn call(&self, function: &str, block: &SubstrateBlock) -> Result<(), InvocationError> {
        self.block_handlers
            .get(function)
            .ok_or_else(|| format!("\"{}\" not found", function))?
            .handle_block(block)
    }
}

struct PluginRegistrar {
    lib: Rc<Library>,
    block_handlers: HashMap<String, BlockHandlerProxy>,
    extrinsic_handlers: HashMap<String, ExtrinsicHandlerProxy>,
    event_handlers: HashMap<String, EventHandlerProxy>,
}

impl PluginRegistrar {
    fn new(lib: Rc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            lib,
            block_handlers: HashMap::default(),
            extrinsic_handlers: HashMap::default(),
            event_handlers: HashMap::default(),
        }
    }
}

impl plugin_core::PluginRegistrar for PluginRegistrar {
    fn register_block_handler(&mut self, name: &str, function: Box<dyn BlockHandler>) {
        let proxy = BlockHandlerProxy {
            function,
            _lib: Rc::clone(&self.lib),
        };
        self.block_handlers.insert(name.to_string(), proxy);
    }

    fn register_extrinsic_handler(&mut self, name: &str, function: Box<dyn ExtrinsicHandler>) {
        let proxy = ExtrinsicHandlerProxy {
            function,
            _lib: Rc::clone(&self.lib),
        };
        self.extrinsic_handlers.insert(name.to_string(), proxy);
    }

    fn register_event_handler(&mut self, name: &str, function: Box<dyn EventHandler>) {
        let proxy = EventHandlerProxy {
            function,
            _lib: Rc::clone(&self.lib),
        };
        self.event_handlers.insert(name.to_string(), proxy);
    }
}

pub struct BlockHandlerProxy {
    function: Box<dyn BlockHandler>,
    _lib: Rc<Library>,
}

impl BlockHandler for BlockHandlerProxy {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), InvocationError> {
        self.function.handle_block(block)
    }
}

pub struct ExtrinsicHandlerProxy {
    function: Box<dyn ExtrinsicHandler>,
    _lib: Rc<Library>,
}

impl ExtrinsicHandler for ExtrinsicHandlerProxy {
    fn handle_extrinsic(&self, extrinsic: &SubstrateExtrinsic) -> Result<(), InvocationError> {
        self.function.handle_extrinsic(extrinsic)
    }
}

pub struct EventHandlerProxy {
    function: Box<dyn EventHandler>,
    _lib: Rc<Library>,
}

impl EventHandler for EventHandlerProxy {
    fn handle_event(&self, event: &SubstrateEvent) -> Result<(), InvocationError> {
        self.function.handle_event(event)
    }
}
