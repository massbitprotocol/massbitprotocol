use crate::core::{
    PluginDeclaration, PluginRegistrar as PluginRegistrarTrait, SolanaBlockHandler,
    SolanaLogMessagesHandler, SolanaTransactionHandler,
};
use index_store::core::Store;
use libloading::Library;
use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use std::{alloc::System, collections::HashMap, error::Error, ffi::OsStr, rc::Rc};

#[global_allocator]
static ALLOCATOR: System = System;

pub struct PluginManager<'a> {
    pub store: &'a dyn Store,
    pub libs: Vec<Rc<Library>>,
    pub solana_block_handlers: HashMap<String, SolanaBlockHandlerProxy>,
    pub solana_transaction_handlers: HashMap<String, SolanaTransactionHandlerProxy>,
    pub solana_event_handlers: HashMap<String, SolanaLogMessagesHandlerProxy>,
}

impl<'a> PluginManager<'a> {
    pub fn new(store: &mut dyn Store) -> PluginManager {
        PluginManager {
            store,
            libs: vec![],
            solana_block_handlers: HashMap::default(),
            solana_transaction_handlers: HashMap::default(),
            solana_event_handlers: HashMap::default(),
        }
    }

    /// Load a plugin library
    /// A plugin library **must** be implemented using the
    /// [`core::plugin_declaration!()`] macro. Trying manually implement
    /// a plugin without going through that macro will result in undefined
    /// behaviour.
    pub unsafe fn load<P: AsRef<OsStr>>(
        &mut self,
        plugin_id: &str,
        library_path: P,
    ) -> Result<(), Box<dyn Error>> {
        let lib = Rc::new(Library::new(library_path)?);

        // inject store to plugin
        lib.get::<*mut Option<&dyn Store>>(b"STORE\0")?
            .write(Some(self.store));

        let plugin_decl = lib
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();
        let mut registrar = PluginRegistrar::new(plugin_id, Rc::clone(&lib));
        (plugin_decl.register)(&mut registrar);

        self.solana_block_handlers
            .extend(registrar.solana_block_handlers);
        self.solana_transaction_handlers
            .extend(registrar.solana_transaction_handlers);
        self.solana_event_handlers
            .extend(registrar.solana_event_handlers);

        self.libs.push(lib);

        Ok(())
    }

    pub fn handle_solana_block(
        &self,
        plugin_id: &str,
        block: &SolanaBlock,
    ) -> Result<(), Box<dyn Error>> {
        self.solana_block_handlers
            .get(plugin_id)
            .ok_or_else(|| format!("\"{}\" not found", plugin_id))?
            .handle_block(block)
    }

    pub fn handle_solana_transaction(
        &self,
        plugin_id: &str,
        transaction: &SolanaTransaction,
    ) -> Result<(), Box<dyn Error>> {
        self.solana_transaction_handlers
            .get(plugin_id)
            .ok_or_else(|| format!("\"{}\" not found", plugin_id))?
            .handle_transaction(transaction)
    }

    pub fn handle_solana_log_messages(
        &self,
        plugin_id: &str,
        event: &SolanaLogMessages,
    ) -> Result<(), Box<dyn Error>> {
        self.solana_event_handlers
            .get(plugin_id)
            .ok_or_else(|| format!("\"{}\" not found", plugin_id))?
            .handle_log_messages(event)
    }
}

struct PluginRegistrar {
    plugin_id: String,
    lib: Rc<Library>,
    solana_block_handlers: HashMap<String, SolanaBlockHandlerProxy>,
    solana_transaction_handlers: HashMap<String, SolanaTransactionHandlerProxy>,
    solana_event_handlers: HashMap<String, SolanaLogMessagesHandlerProxy>,
}

impl PluginRegistrar {
    fn new(plugin_id: &str, lib: Rc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            plugin_id: plugin_id.to_string(),
            lib,
            solana_block_handlers: HashMap::default(),
            solana_transaction_handlers: HashMap::default(),
            solana_event_handlers: HashMap::default(),
        }
    }
}

impl PluginRegistrarTrait for PluginRegistrar {
    fn register_solana_block_handler(&mut self, handler: Box<dyn SolanaBlockHandler>) {
        let proxy = SolanaBlockHandlerProxy {
            handler,
            _lib: Rc::clone(&self.lib),
        };
        self.solana_block_handlers
            .insert(self.plugin_id.clone(), proxy);
    }

    fn register_solana_transaction_handler(&mut self, handler: Box<dyn SolanaTransactionHandler>) {
        let proxy = SolanaTransactionHandlerProxy {
            handler,
            _lib: Rc::clone(&self.lib),
        };
        self.solana_transaction_handlers
            .insert(self.plugin_id.clone(), proxy);
    }

    fn register_solana_event_handler(&mut self, handler: Box<dyn SolanaLogMessagesHandler>) {
        let proxy = SolanaLogMessagesHandlerProxy {
            handler,
            _lib: Rc::clone(&self.lib),
        };
        self.solana_event_handlers
            .insert(self.plugin_id.clone(), proxy);
    }
}

pub struct SolanaBlockHandlerProxy {
    handler: Box<dyn SolanaBlockHandler>,
    _lib: Rc<Library>,
}

impl SolanaBlockHandler for SolanaBlockHandlerProxy {
    fn handle_block(&self, block: &SolanaBlock) -> Result<(), Box<dyn Error>> {
        self.handler.handle_block(block)
    }
}

pub struct SolanaTransactionHandlerProxy {
    handler: Box<dyn SolanaTransactionHandler>,
    _lib: Rc<Library>,
}

impl SolanaTransactionHandler for SolanaTransactionHandlerProxy {
    fn handle_transaction(&self, transaction: &SolanaTransaction) -> Result<(), Box<dyn Error>> {
        self.handler.handle_transaction(transaction)
    }
}

pub struct SolanaLogMessagesHandlerProxy {
    handler: Box<dyn SolanaLogMessagesHandler>,
    _lib: Rc<Library>,
}

impl SolanaLogMessagesHandler for SolanaLogMessagesHandlerProxy {
    fn handle_log_messages(&self, event: &SolanaLogMessages) -> Result<(), Box<dyn Error>> {
        self.handler.handle_log_messages(event)
    }
}
