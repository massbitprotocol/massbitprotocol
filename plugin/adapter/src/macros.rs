//extern crate proc_macro;
//use proc_macro::{self, TokenStream};
//use syn::{parse_macro_input, DataEnum, DataUnion, DeriveInput, FieldsNamed, FieldsUnnamed};
///
/// Create adapter
/// For example create_adapter!("Solana", {
///     handle_block:SolanaBlock,
///     handle_transaction:SolanaTransaction,
///     handle_log_messages:SolanaLogMessages
/// });
///
#[macro_export]
macro_rules! prepare_adapter {
    ($adapter:ident, { $($method:ident : $msgtype:ident),*}) => {
        paste! {
            lazy_static::lazy_static! {
                static ref COMPONENT_NAME: String = String::from(format!("[{}-Adapter]", quote::quote!($adapter)));
            }
            pub trait [<$adapter Handler>] {
                $(
                fn $method(&self, _message: &$msgtype) -> Result<(), Box<dyn Error>> {
                    Ok(())
                }
                )*
            }
            /// A proxy object which wraps a [`Handler`] and makes sure it can't outlive
            /// the library it came from.
            pub struct [<$adapter HandlerProxy>] {
                pub handler: Box<dyn [<$adapter Handler>] + Send + Sync>,
                _lib: Arc<Library>,
            }
            impl [<$adapter HandlerProxy>] {
                pub fn new(handler: Box<dyn [<$adapter Handler>] + Send + Sync>, _lib: Arc<Library>) -> [<$adapter HandlerProxy>] {
                    [<$adapter HandlerProxy>] {
                        handler,
                        _lib
                    }
                }
            }
            impl [<$adapter Handler>] for [<$adapter HandlerProxy>] {
                $(
                fn $method(&self, message: &$msgtype) -> Result<(), Box<dyn Error>> {
                    self.handler.$method(message)
                }
                )*
            }
        }
    };
}

#[macro_export]
macro_rules! create_adapters {
    ($($adapter:ident),*) => {
        paste! {

            $(
            pub mod [<$adapter:lower>];
            )*
            use crate::{$([<$adapter:lower>]::*),*};
            pub enum HandlerProxyType {
                $(
                    $adapter([<$adapter HandlerProxy>])
                ),*
            }

            pub trait PluginRegistrar {
                $(
                fn [<register_ $adapter:lower _handler>](&mut self, handler: Box<dyn [<$adapter Handler>] + Send + Sync>);
                )*
            }

            impl PluginRegistrar for AdapterHandler {
                $(
                fn [<register_ $adapter:lower _handler>](&mut self, handler: Box<dyn [<$adapter Handler>] + Send + Sync>) {
                    self.handler_proxies.insert(
                            format!("{}", quote!([<$adapter:lower>])),
                            HandlerProxyType::$adapter([<$adapter HandlerProxy>]::new(handler, Arc::clone(&self.lib))));

                }
                )*
            }

            pub fn handle_message(proxy_type: &HandlerProxyType, message : &mut GenericDataProto) -> Result<(), Box<dyn Error>> {
                match proxy_type {
                    $(
                    HandlerProxyType::$adapter(proxy) => {
                            proxy.handle_message(message)
                        }
                    )*
                }
            }
        }
    }
}

/*
#[macro_export]
macro_rules! create_adapters0 {
    ($($adapter:ident { $($method:ident : $msgtype:ident),*}),*) => {
        paste! {
            $(pub mod [<$adapter:lower>] {
                use std::{error::Error, fmt, sync::Arc};
                use libloading::Library;
                use quote::quote;
                lazy_static::lazy_static! {
                    static ref COMPONENT_NAME: String = String::from(format!("[{}-Adapter]", quote::quote!($adapter)));
                }
                $(
                pub trait [<$adapter $method:camel Handler>] {
                    fn $method(&self, message: &$msgtype) -> Result<(), Box<dyn Error>>;
                }
                /// A proxy object which wraps a [`Handler`] and makes sure it can't outlive
                /// the library it came from.
                pub struct [<$adapter $method:camel HandlerProxy>] {
                    pub handler: Box<dyn [<$adapter $method:camel Handler>] + Send + Sync>,
                    _lib: Arc<Library>,
                }
                impl [<$adapter $method:camel HandlerProxy>] {
                    pub fn new(handler: Box<dyn [<$adapter $method:camel Handler>] + Send + Sync>, _lib: Arc<Library>) -> [<$adapter $method:camel HandlerProxy>] {
                        [<$adapter $method:camel HandlerProxy>] {
                            handler,
                            _lib
                        }
                    }
                }
                impl [<$adapter $method:camel Handler>] for [<$adapter $method:camel HandlerProxy>] {
                    fn $method(&self, message: &$msgtype) -> Result<(), Box<dyn Error>> {
                        self.handler.$method(message)
                    }
                }
                )*
                pub trait [<$adapter Registrar>] {
                    $(
                    fn [<register_ $adapter:lower _ $method>](&mut self, handler: Box<dyn [<$adapter $method:camel Handler>] + Send + Sync>);
                    )*
                }

                pub struct [<$adapter Handler>] {
                    indexer_hash: String,
                    def_index: String,
                    lib: Arc<Library>,
                    $(
                    [<$method _handlers>]: HashMap<String, [<$adapter $method:camel HandlerProxy>]>,
                    )*
                }

                impl [<$adapter Handler>] {
                    fn new(hash: String, lib: Arc<Library>) -> [<$adapter Handler>] {
                         [<$adapter Handler>] {
                            indexer_hash: hash,
                            def_index:"_".to_string(),
                            lib,
                            $(
                            [<$method _handlers>]: HashMap::default(),
                            )*
                        }
                    }
                }
                impl [<$adapter Registrar>] for [<$adapter Handler>] {
                    $(
                    fn [<register_ $adapter:lower _ $method>](&mut self, handler: Box<dyn [<$adapter $method:camel Handler>] + Send + Sync>) {
                        self.[<$method _handlers>].insert(self.def_index.clone(), [<$adapter $method:camel HandlerProxy>]::new(handler, Arc::clone(&self.lib)));
                    }
                    )*
                }
                $(
                impl [<$adapter $method:camel Handler>] for [<$adapter Handler>] {
                    fn $method(&self, message: &$msgtype) -> Result<(), Box<dyn Error>> {
                        self.[<$method _handlers>].get(&self.def_index)
                            .ok_or_else(|| format!("Handler for method \"{}\" not found", quote!($method)))?
                            .$method(message);
                    }
                }
                )*
            }
            )*
            pub trait AdapterRegistrar {
                $($(
                    fn [<register_ $adapter:lower _ $method>](&mut self, handler: Box<dyn [<$adapter:lower>]::[<$adapter $method:camel Handler>] + Send + Sync>);
                )*)*
            }
            impl AdapterRegistrar for AdapterHandler {
                $($(
                fn [<register_ $adapter:lower _ $method>](&mut self, handler: Box<dyn [<$adapter:lower>]::[<$adapter $method:camel Handler>] + Send + Sync>) {
                    println!("Register method {}", format!("{}_{}",quote!($adapter),quote!($method)));
                }
                )*)*
            }
        }
    };
}
*/

/*
#[macro_export]
macro_rules! registrar {
    ($($adapter:ident),*) => {
        paste! {
            $(
            impl [<$adapter RegistrarTrait>] for AdapterRegistrar {
                fn [<register_ $adapter:lower _handler>](&mut self, handler: Box<dyn [<$adapter Handler>]>) {
                    let proxy = [<$adapter HandlerProxy>]::new(handler, Arc::clone(&self.lib));
                    self.[<$adapter:lower _handler_proxies>].insert(self.adapter_id.clone(), proxy);
                }
            }
            )*
        }
    }
}
*/

/*
#[proc_macro_attribute]
pub fn add_field(_args: TokenStream, input: TokenStream) -> TokenStream  {
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    fields
                        .named
                        .push(syn::Field::parse_named.parse2(quote! { pub a: String }).unwrap());
                }
                _ => {
                    ()
                }
            }

            return quote! {
                #ast
            }.into();
        }
        _ => panic!("`add_field` has to be used with structs "),
    }
}
 */