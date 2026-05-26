//! Plugin loader integration tests (TDD-2).
//!
//! Requires mock plugin cdylib exporting:
//!   `gkit_plugin_abi_version() -> u32` (returns 1)
//!   `create_mock_backend() -> u32`   (returns 42)
//!
//! Run: `cargo build -p gkit-mock-plugin && cargo test -p gkit-core --test plugin_loader`

use libloading::{Library, Symbol};
use stabby::libloading::StabbyLibrary;
use std::path::PathBuf;

fn mock_plugin_path() -> PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let ws_root = manifest_dir
        .parent()
        .expect("should be in crates/")
        .parent()
        .expect("should be in workspace root");

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    let lib_name = if cfg!(target_os = "macos") {
        "libgkit_mock_plugin.dylib"
    } else if cfg!(target_os = "linux") {
        "libgkit_mock_plugin.so"
    } else if cfg!(target_os = "windows") {
        "gkit_mock_plugin.dll"
    } else {
        "libgkit_mock_plugin.so"
    };

    ws_root.join("target").join(profile).join(lib_name)
}

#[test]
#[ignore = "requires mock_plugin built via cargo build -p gkit-mock-plugin"]
fn load_mock_plugin_gets_abi_version() {
    // SAFETY: mock plugin is a trusted cdylib without initialization side effects.
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    // SAFETY: gkit_plugin_abi_version has the declared signature.
    let version: Symbol<extern "C" fn() -> u32> =
        unsafe { lib.get(b"gkit_plugin_abi_version").unwrap() };
    assert_eq!(version(), 1);
}

#[test]
#[ignore = "requires mock_plugin built via cargo build -p gkit-mock-plugin"]
fn load_mock_plugin_with_stabby_type_check() {
    // SAFETY: mock plugin is a trusted cdylib without initialization side effects.
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    // SAFETY: `create_mock_backend` is annotated with #[stabby::export(canaries)].
    let create = unsafe {
        lib.get_stabbied::<extern "C" fn() -> u32>(b"create_mock_backend")
    }
    .unwrap();
    assert_eq!(create(), 42);
}

#[test]
#[ignore = "requires mock_plugin built via cargo build -p gkit-mock-plugin"]
fn abi_version_mismatch_is_detected() {
    // SAFETY: mock plugin is a trusted cdylib without initialization side effects.
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    // SAFETY: gkit_plugin_abi_version has the declared signature.
    let version: Symbol<extern "C" fn() -> u32> =
        unsafe { lib.get(b"gkit_plugin_abi_version").unwrap() };
    assert_ne!(version(), 999);
}

#[test]
#[ignore = "requires mock_plugin built via cargo build -p gkit-mock-plugin"]
fn missing_symbol_returns_error() {
    // SAFETY: mock plugin is a trusted cdylib without initialization side effects.
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    let result: Result<Symbol<extern "C" fn() -> u32>, _> =
        unsafe { lib.get(b"nonexistent_symbol") };
    assert!(result.is_err());
}
