//! Mock plugin for testing the gkit-core plugin loader.
//!
//! Exports:
//! - `gkit_plugin_abi_version()` → 1 (must match `gkit_core::plugin::loader::ABI_VERSION`)
//! - `create_mock_backend()` → 42 (arbitrary mock backend identifier)

#[stabby::export]
pub extern "C" fn gkit_plugin_abi_version() -> u32 {
    1
}

#[stabby::export(canaries)]
pub extern "C" fn create_mock_backend() -> u32 {
    42
}
