use crate::core::{
    PluginDeclaration, PluginRegistrar as PluginRegistrarTrait, SolanaBlockHandler,
    SolanaLogMessagesHandler, SolanaTransactionHandler, SubstrateBlockHandler, SubstrateEventHandler,
    SubstrateExtrinsicHandler,
};
use index_store::core::Store;
use libloading::Library;
use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use massbit_chain_substrate::data_type::{
    SubstrateBlock, SubstrateCheckedExtrinsic, SubstrateEventRecord, SubstrateUncheckedExtrinsic
};
use std::{alloc::System, collections::HashMap, error::Error, ffi::OsStr, rc::Rc};

#[global_allocator]
static ALLOCATOR: System = System;

pub struct PluginManager<'a> {
    pub store: &'a dyn Store,
    pub libs: Vec<Rc<Library>>,
    pub substrate_block_handlers: HashMap<String, SubstrateBlockHandlerProxy>,
    pub substrate_extrinsic_handlers: HashMap<String, SubstrateExtrinsicHandlerProxy>,
    pub substrate_event_handlers: HashMap<String, SubstrateEventHandlerProxy>,
    pub solana_block_handlers: HashMap<String, SolanaBlockHandlerProxy>,
    pub solana_transaction_handlers: HashMap<String, SolanaTransactionHandlerProxy>,
    pub solana_event_handlers: HashMap<String, SolanaLogMessagesHandlerProxy>,
}

impl<'a> PluginManager<'a> {
    pub fn new(store: &dyn Store) -> PluginManager {
        PluginManager {
            store,
            libs: vec![],
            substrate_block_handlers: HashMap::default(),
            substrate_extrinsic_handlers: HashMap::default(),
            substrate_event_handlers: HashMap::default(),
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

        self.substrate_block_handlers
            .extend(registrar.substrate_block_handlers);
        self.substrate_extrinsic_handlers
            .extend(registrar.substrate_extrinsic_handlers);
        self.substrate_event_handlers
            .extend(registrar.substrate_event_handlers);
        self.solana_block_handlers
            .extend(registrar.solana_block_handlers);
        self.solana_transaction_handlers
            .extend(registrar.solana_transaction_handlers);
        self.solana_event_handlers
            .extend(registrar.solana_event_handlers);

        self.libs.push(lib);

        Ok(())
    }

    pub fn handle_substrate_block(
        &self,
        plugin_id: &str,
        block: &SubstrateBlock,
    ) -> Result<(), Box<dyn Error>> {
        self.substrate_block_handlers
            .get(plugin_id)
            .ok_or_else(|| format!("\"{}\" not found", plugin_id))?
            .handle_block(block)
    }

    pub fn handle_substrate_extrinsic(
        &self,
        plugin_id: &str,
        extrinsic: &SubstrateUncheckedExtrinsic,
    ) -> Result<(), Box<dyn Error>> {
        self.substrate_extrinsic_handlers
            .get(plugin_id)
            .ok_or_else(|| format!("\"{}\" not found", plugin_id))?
            .handle_extrinsic(extrinsic)
    }

    pub fn handle_substrate_event(
        &self,
        plugin_id: &str,
        event: &SubstrateEventRecord,
    ) -> Result<(), Box<dyn Error>> {
        self.substrate_event_handlers
            .get(plugin_id)
            .ok_or_else(|| format!("\"{}\" not found", plugin_id))?
            .handle_event(event)
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
    substrate_block_handlers: HashMap<String, SubstrateBlockHandlerProxy>,
    substrate_extrinsic_handlers: HashMap<String, SubstrateExtrinsicHandlerProxy>,
    substrate_event_handlers: HashMap<String, SubstrateEventHandlerProxy>,
    solana_block_handlers: HashMap<String, SolanaBlockHandlerProxy>,
    solana_transaction_handlers: HashMap<String, SolanaTransactionHandlerProxy>,
    solana_event_handlers: HashMap<String, SolanaLogMessagesHandlerProxy>,
}

impl PluginRegistrar {
    fn new(plugin_id: &str, lib: Rc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            plugin_id: plugin_id.to_string(),
            lib,
            substrate_block_handlers: HashMap::default(),
            substrate_extrinsic_handlers: HashMap::default(),
            substrate_event_handlers: HashMap::default(),
            solana_block_handlers: HashMap::default(),
            solana_transaction_handlers: HashMap::default(),
            solana_event_handlers: HashMap::default(),
        }
    }
}

impl PluginRegistrarTrait for PluginRegistrar {
    fn register_substrate_block_handler(&mut self, handler: Box<dyn SubstrateBlockHandler>) {
        let proxy = SubstrateBlockHandlerProxy {
            handler,
            _lib: Rc::clone(&self.lib),
        };
        self.substrate_block_handlers
            .insert(self.plugin_id.clone(), proxy);
    }

    fn register_substrate_extrinsic_handler(
        &mut self,
        handler: Box<dyn SubstrateExtrinsicHandler>,
    ) {
        let proxy = SubstrateExtrinsicHandlerProxy {
            handler,
            _lib: Rc::clone(&self.lib),
        };
        self.substrate_extrinsic_handlers
            .insert(self.plugin_id.clone(), proxy);
    }

    fn register_substrate_event_handler(&mut self, handler: Box<dyn SubstrateEventHandler>) {
        let proxy = SubstrateEventHandlerProxy {
            handler,
            _lib: Rc::clone(&self.lib),
        };
        self.substrate_event_handlers
            .insert(self.plugin_id.clone(), proxy);
    }

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

/// A proxy object which wraps a [`Handler`] and makes sure it can't outlive
/// the library it came from.
pub struct SubstrateBlockHandlerProxy {
    handler: Box<dyn SubstrateBlockHandler>,
    _lib: Rc<Library>,
}

impl SubstrateBlockHandler for SubstrateBlockHandlerProxy {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), Box<dyn Error>> {
        self.handler.handle_block(block)
    }
}

pub struct SubstrateExtrinsicHandlerProxy {
    handler: Box<dyn SubstrateExtrinsicHandler>,
    _lib: Rc<Library>,
}

impl SubstrateExtrinsicHandler for SubstrateExtrinsicHandlerProxy {
    fn handle_extrinsic(
        &self,
        // extrinsic: &SubstrateCheckedExtrinsic,
        extrinsic: &SubstrateUncheckedExtrinsic,
    ) -> Result<(), Box<dyn Error>> {
        self.handler.handle_extrinsic(extrinsic)
    }
}

pub struct SubstrateEventHandlerProxy {
    handler: Box<dyn SubstrateEventHandler>,
    _lib: Rc<Library>,
}

impl SubstrateEventHandler for SubstrateEventHandlerProxy {
    fn handle_event(&self, event: &SubstrateEventRecord) -> Result<(), Box<dyn Error>> {
        self.handler.handle_event(event)
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
