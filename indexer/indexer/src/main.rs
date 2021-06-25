#[macro_use]
extern crate diesel_derive_table;
#[macro_use]
extern crate diesel;

use diesel::pg::Pg;
use diesel::prelude::*;
use libloading::Library;
use plugins_core::{Function, InvocationError, PluginDeclaration};
use std::{alloc::System, collections::HashMap, env, ffi::OsStr, io, path::PathBuf, rc::Rc};
use types::SubstrateBlock;

#[global_allocator]
static ALLOCATOR: System = System;

fn main() {
    let block = SubstrateBlock { idx: 1 };

    let args = env::args().skip(1);
    let args = Args::parse(args).expect("Usage: app <plugin-path> <function> <args>...");

    let mut functions = ExternalFunctions::new();
    unsafe {
        functions
            .load(&args.plugin_library)
            .expect("Function loading failed");
    }

    let _ = functions
        .call(&args.function, &block)
        .expect("Invocation failed");
}

struct Args {
    plugin_library: PathBuf,
    function: String,
}

impl Args {
    fn parse(mut args: impl Iterator<Item = String>) -> Option<Args> {
        let plugin_library = PathBuf::from(args.next()?);
        let function = args.next()?;
        Some(Args {
            plugin_library,
            function,
        })
    }
}

/// A map of all externally provided functions.
#[derive(Default)]
pub struct ExternalFunctions {
    functions: HashMap<String, FunctionProxy>,
    libraries: Vec<Rc<Library>>,
}

impl ExternalFunctions {
    pub fn new() -> ExternalFunctions {
        ExternalFunctions::default()
    }

    pub fn call(&self, function: &str, block: &SubstrateBlock) -> Result<(), InvocationError> {
        self.functions
            .get(function)
            .ok_or_else(|| format!("\"{}\" not found", function))?
            .handle_block(block)
    }

    pub unsafe fn load<P: AsRef<OsStr>>(&mut self, library_path: P) -> io::Result<()> {
        // load the library into memory
        let library = Rc::new(Library::new(library_path)?);

        // get a pointer to the plugin_declaration symbol.
        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();

        let mut registrar = PluginRegistrar::new(Rc::clone(&library));

        (decl.register)(&mut registrar);

        // add all loaded plugins to the functions map
        self.functions.extend(registrar.functions);
        // and make sure ExternalFunctions keeps a reference to the library
        self.libraries.push(library);

        Ok(())
    }
}

struct PluginRegistrar {
    functions: HashMap<String, FunctionProxy>,
    lib: Rc<Library>,
}

impl PluginRegistrar {
    fn new(lib: Rc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            lib,
            functions: HashMap::default(),
        }
    }
}

impl plugins_core::PluginRegistrar for PluginRegistrar {
    fn register_function(&mut self, name: &str, function: Box<dyn Function>) {
        let proxy = FunctionProxy {
            function,
            _lib: Rc::clone(&self.lib),
        };
        self.functions.insert(name.to_string(), proxy);
    }
}

pub struct FunctionProxy {
    function: Box<dyn Function>,
    _lib: Rc<Library>,
}

impl Function for FunctionProxy {
    fn handle_block(&self, block: &SubstrateBlock) -> Result<(), InvocationError> {
        self.function.handle_block(block)
    }
}
