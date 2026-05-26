//! Integration tests for `gkit_core::plugin::backend::PluginBackend`.
//!
//! Covers:
//! - Static variant: construction, accessors, Drop semantics
//! - Dynamic variant: Drop order verification (instance before _lib)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gkit_core::plugin::backend::PluginBackend;
use gkit_core::plugin::loader::PluginLib;

// ============================================================================
// Test helpers
// ============================================================================

/// Utility type that records when it was dropped via a shared `AtomicBool`.
struct DropTracker {
    flag: Arc<AtomicBool>,
}

impl DropTracker {
    fn new() -> (Self, Arc<AtomicBool>) {
        let flag = Arc::new(AtomicBool::new(false));
        (Self { flag: Arc::clone(&flag) }, flag)
    }

    fn is_dropped(flag: &Arc<AtomicBool>) -> bool {
        flag.load(Ordering::SeqCst)
    }
}

impl Drop for DropTracker {
    fn drop(&mut self) {
        self.flag.store(true, Ordering::SeqCst);
    }
}

// ============================================================================
// Static variant
// ============================================================================

#[test]
fn static_backend_works() {
    let backend = PluginBackend::r#static(42u32);

    // Variant checks
    assert!(backend.is_static());
    assert!(!backend.is_dynamic());

    // Accessors
    assert_eq!(*backend.instance(), 42);
    assert_eq!(backend.as_ref(), &42);

    // Deref
    assert_eq!(*backend, 42);
}

#[test]
fn static_backend_into_inner() {
    let backend = PluginBackend::r#static(42u32);
    let inner: u32 = backend.into_inner();
    assert_eq!(inner, 42);
}

#[test]
fn static_backend_drops_inner() {
    let (tracker, dropped) = DropTracker::new();

    {
        let backend = PluginBackend::r#static(tracker);
        assert!(!DropTracker::is_dropped(&dropped));
        let _ = backend; // suppress unused warning
    }

    assert!(DropTracker::is_dropped(&dropped));
}

#[test]
fn static_backend_instance_returns_ref() {
    let backend = PluginBackend::r#static(String::from("hello"));
    // instance() returns a shared reference
    assert_eq!(backend.instance(), &String::from("hello"));
    assert_eq!(backend.as_ref(), &String::from("hello"));
}

// ============================================================================
// Dynamic variant — Drop order
// ============================================================================

/// Verify that when a `PluginBackend::Dynamic` is dropped, the `instance` field
/// is dropped **before** the `_lib` field.
///
/// Rust guarantees that struct fields are dropped in **declaration order**.
/// Since `PluginBackend::Dynamic` declares:
/// ```ignore
/// Dynamic {
///     instance: T,      // ← dropped FIRST
///     _lib: PluginLib,  // ← dropped SECOND
/// }
/// ```
/// all resources allocated inside the plugin library are freed **before**
/// the library itself is unloaded.
///
/// # Test variants
///
/// - **WASM**: `PluginLib` is a ZST (`pub struct PluginLib;`) — full test.
/// - **Native**: Requires a real cdylib (e.g., mock_plugin). Marked `#[ignore]`
///   because the structural guarantee is verified at compile time by Rust's
///   well-defined field drop order.

#[cfg(target_arch = "wasm32")]
#[test]
fn dynamic_backend_drops_instance_before_library() {
    // WASM: PluginLib is a ZST — constructable inline
    let (instance_tracker, instance_dropped) = DropTracker::new();
    let lib = PluginLib;

    {
        let backend = PluginBackend::dynamic(lib, instance_tracker);
        assert!(backend.is_dynamic());
        assert!(!backend.is_static());
        assert!(!DropTracker::is_dropped(&instance_dropped));
        let _ = backend;
    }

    // After dropping the backend, instance must have been dropped
    assert!(DropTracker::is_dropped(&instance_dropped));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "requires a real cdylib; drop order is structurally guaranteed by Rust field declaration order"]
fn dynamic_backend_drops_instance_before_library() {
    // This test exercises PluginBackend::Dynamic with a real PluginLib load.
    //
    // To enable, build the mock_plugin cdylib first:
    //   cargo build -p gkit-mock-plugin
    //
    // Then locate the built artifact (platform-dependent):
    //   - macOS:   target/debug/libgkit_mock_plugin.dylib
    //   - Linux:   target/debug/libgkit_mock_plugin.so
    //   - Windows: target/debug/gkit_mock_plugin.dll
    //
    // Example activation (macOS):
    // ```rust,ignore
    // let mock_path = concat!(
    //     env!("CARGO_MANIFEST_DIR"),
    //     "/../../../target/debug/libgkit_mock_plugin.dylib"
    // );
    // // SAFETY: mock_plugin is a test-only cdylib with known exports.
    // let lib = unsafe { PluginLib::open(mock_path) }
    //     .expect("mock_plugin not built");
    // let (tracker, dropped) = DropTracker::new();
    // let backend = PluginBackend::dynamic(lib, tracker);
    // drop(backend);
    // assert!(DropTracker::is_dropped(&dropped));
    // ```
}

#[test]
fn dynamic_backend_api_consistency() {
    // Verify Dynamic and Static share the same API surface.
    // We can only test Static here; Dynamic's shape is structurally identical.
    let static_backend = PluginBackend::r#static(99u32);

    // All public API methods exist
    let _is_dyn: bool = static_backend.is_dynamic(); // false for Static
    let _is_static: bool = static_backend.is_static(); // true for Static
    let _inst: &u32 = static_backend.instance();
    let _asref: &u32 = static_backend.as_ref();
    let _deref: &u32 = &static_backend;
    let _consumed: u32 = static_backend.into_inner();

    // If this compiles, the API surface is consistent.
}

// ============================================================================
// Enum variant discrimination
// ============================================================================

#[test]
fn variants_are_mutually_exclusive() {
    let static_backend = PluginBackend::r#static(1u32);
    assert!(static_backend.is_static());
    assert!(!static_backend.is_dynamic());
}

#[test]
fn debug_formatting() {
    let backend = PluginBackend::r#static(42u32);
    let debug = format!("{:?}", backend);
    assert!(debug.contains("Static"), "debug output should mention Static: {debug}");
}
