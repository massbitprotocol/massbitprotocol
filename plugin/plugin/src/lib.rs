pub mod core;
pub mod manager;

pub use manager::PluginManager;

#[macro_export]
macro_rules! export_plugin {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static plugin_declaration: $crate::core::PluginDeclaration =
            $crate::core::PluginDeclaration {
                register: $register,
            };
    };
}
