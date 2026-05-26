use super::loader::PluginLib;
use std::ops::Deref;

// ===========================================================================
// PluginBackend<T> — Dynamic vs Static backend dispatch
// ===========================================================================

/// Represents a loaded plugin backend.
///
/// # Variants
///
/// * `Dynamic { _lib, instance }` — Loaded at runtime from a cdylib via `libloading`.
/// * `Static(instance)` — Compile-time linked (WASM, or directly linked native).
///
/// # Drop Ordering (Critical)
///
/// For the `Dynamic` variant, Rust drops fields in **declaration order**:
/// `instance` is dropped first, then `_lib`. This guarantees that all resources
/// allocated inside the library are freed before the library itself is unloaded.
///
/// The field name `_lib` has a leading underscore to suppress the "unused field"
/// compiler warning since access always goes through the public API methods.
///
/// # Type Parameters
///
/// * `T` — The concrete backend instance (e.g., `Box<dyn PeerConnectionFactory>`).
pub enum PluginBackend<T> {
    /// Loaded at runtime from a cdylib via `libloading`.
    Dynamic {
        /// The actual backend instance created by the plugin. Dropped FIRST.
        instance: T,
        /// RAII guard keeping the cdylib loaded. Dropped AFTER `instance`.
        /// Leading underscore: field intentionally not read directly (accessed via `instance()`).
        _lib: PluginLib,
    },
    /// Statically registered at compile time (WASM, or linked directly).
    Static(T),
}

impl<T> PluginBackend<T> {
    /// Returns a reference to the backend instance.
    ///
    /// Works uniformly for both `Dynamic` and `Static` variants.
    #[must_use]
    pub fn instance(&self) -> &T {
        match self {
            Self::Dynamic { instance, .. } => instance,
            Self::Static(instance) => instance,
        }
    }

    /// Returns `true` if this backend was dynamically loaded from a cdylib.
    #[must_use]
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic { .. })
    }

    /// Returns `true` if this backend was statically linked at compile time.
    #[must_use]
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static(_))
    }

    /// Consumes the `PluginBackend` and returns the inner instance.
    ///
    /// For the `Dynamic` variant, this drops the library reference, meaning the
    /// cdylib may be unloaded after the returned value is dropped (assuming no
    /// other references to the same library exist).
    #[must_use]
    pub fn into_inner(self) -> T {
        match self {
            Self::Dynamic { instance, .. } => instance,
            Self::Static(instance) => instance,
        }
    }

    // -----------------------------------------------------------------------
    // Convenience constructors
    // -----------------------------------------------------------------------

    /// Create a new `Dynamic` backend from an already-loaded library and instance.
    #[must_use]
    pub fn dynamic(lib: PluginLib, instance: T) -> Self {
        Self::Dynamic {
            _lib: lib,
            instance,
        }
    }

    /// Create a new `Static` backend.
    #[must_use]
    pub fn r#static(instance: T) -> Self {
        Self::Static(instance)
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl<T> Deref for PluginBackend<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.instance()
    }
}

impl<T> AsRef<T> for PluginBackend<T> {
    fn as_ref(&self) -> &T {
        self.instance()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for PluginBackend<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dynamic { instance, .. } => {
                f.debug_struct("Dynamic")
                    .field("instance", instance)
                    .finish()
            }
            Self::Static(instance) => {
                f.debug_struct("Static")
                    .field("instance", instance)
                    .finish()
            }
        }
    }
}
