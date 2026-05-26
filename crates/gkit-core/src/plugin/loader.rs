use std::path::Path;
use std::sync::Arc;

use stabby::libloading::StabbyLibrary;

use crate::plugin::error::{PluginError, PluginResult};

/// Host ABI version. Plugins must export `gkit_plugin_abi_version()` returning this value.
pub const ABI_VERSION: u32 = 1;

/// Symbol name for the ABI version check entry point.
pub const ABI_VERSION_SYM: &[u8] = b"gkit_plugin_abi_version";

// ============================================================================
// PluginLib — Wraps libloading::Library
// ============================================================================

/// Handle to a dynamically loaded plugin library.
///
/// Wraps an `Arc<libloading::Library>`, keeping the shared library loaded for
/// the process lifetime. Multiple backends can share the same library via clone.
/// On WASM this is a ZST since dynamic loading is unsupported.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct PluginLib(Arc<libloading::Library>);

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct PluginLib;

#[cfg(not(target_arch = "wasm32"))]
impl PluginLib {
    /// # Safety
    ///
    /// The library at `path` may execute arbitrary code during initialization.
    /// Caller must ensure the library is trusted and well-formed.
    pub unsafe fn open<P: AsRef<Path>>(path: P) -> PluginResult<Self> {
        let lib = unsafe { libloading::Library::new(path.as_ref()) }.map_err(|source| {
            let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(source);
            PluginError::LoadFailed {
                path: path.as_ref().to_path_buf(),
                source: boxed,
            }
        })?;
        Ok(PluginLib(Arc::new(lib)))
    }

    #[must_use]
    pub fn from_library(lib: libloading::Library) -> Self {
        PluginLib(Arc::new(lib))
    }

    /// # Safety
    ///
    /// Caller must ensure the symbol has the correct type `T`. The plugin may
    /// execute arbitrary code when the returned function pointer is called.
    pub unsafe fn get_symbol<T>(&self, symbol: &[u8]) -> PluginResult<libloading::Symbol<'_, T>> {
        unsafe { self.0.get::<T>(symbol) }.map_err(|_| PluginError::MissingSymbol {
            name: "plugin".to_string(),
            symbol: String::from_utf8_lossy(symbol).to_string(),
        })
    }

    /// # Safety
    ///
    /// Caller must ensure `T` matches the type declared in the plugin's
    /// `#[stabby::export(canaries)]` annotation. Type mismatch will be caught
    /// by the canary check, but symbol name mismatch is undefined behavior.
    pub unsafe fn get_stabbied<T: stabby::IStable>(
        &self,
        symbol: &[u8],
    ) -> PluginResult<stabby::libloading::Symbol<'_, T>> {
        unsafe { self.0.get_stabbied::<T>(symbol) }.map_err(|_| PluginError::MissingSymbol {
            name: "plugin".to_string(),
            symbol: String::from_utf8_lossy(symbol).to_string(),
        })
    }

    /// # Safety
    ///
    /// The plugin library must export a valid `gkit_plugin_abi_version` symbol.
    /// Calling the version function invokes foreign code.
    pub unsafe fn check_abi_version(&self) -> PluginResult<()> {
        let version_fn: libloading::Symbol<extern "C" fn() -> u32> =
            unsafe { self.0.get(ABI_VERSION_SYM) }.map_err(|_| PluginError::MissingSymbol {
                name: "plugin".to_string(),
                symbol: String::from_utf8_lossy(ABI_VERSION_SYM).to_string(),
            })?;

        let plugin_version = version_fn();
        if plugin_version != ABI_VERSION {
            return Err(PluginError::AbiVersionMismatch {
                plugin: plugin_version,
                host: ABI_VERSION,
            });
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Debug for PluginLib {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginLib").finish_non_exhaustive()
    }
}

#[cfg(target_arch = "wasm32")]
impl PluginLib {
    pub unsafe fn open<P: AsRef<Path>>(_path: P) -> PluginResult<Self> {
        Ok(PluginLib)
    }

    #[must_use]
    pub fn from_library(_lib: ()) -> Self {
        PluginLib
    }

    pub unsafe fn get_symbol<T>(&self, _symbol: &[u8]) -> PluginResult<T> {
        Err(PluginError::MissingSymbol {
            name: "plugin".to_string(),
            symbol: "dynamic loading unsupported on WASM".to_string(),
        })
    }

    pub unsafe fn get_stabbied<T: stabby::IStable>(
        &self,
        _symbol: &[u8],
    ) -> PluginResult<stabby::libloading::Symbol<'_, T>> {
        Err(PluginError::MissingSymbol {
            name: "plugin".to_string(),
            symbol: "dynamic loading unsupported on WASM".to_string(),
        })
    }

    pub unsafe fn check_abi_version(&self) -> PluginResult<()> {
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl std::fmt::Debug for PluginLib {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginLib (WASM ZST)").finish()
    }
}

// ============================================================================
// PluginLoader — orchestration
// ============================================================================

pub struct PluginLoader;

impl PluginLoader {
    /// # Safety
    ///
    /// The dynamic library at `path` will execute arbitrary code during
    /// `PluginLib::open`. The `factory_fn` closure must load plugin symbols
    /// with the correct types.
    pub unsafe fn load<T, P: AsRef<Path>, F>(path: P, factory_fn: F) -> PluginResult<(PluginLib, T)>
    where
        F: FnOnce(&PluginLib) -> PluginResult<T>,
    {
        let lib = unsafe { PluginLib::open(path) }?;
        unsafe { lib.check_abi_version() }?;
        let instance = factory_fn(&lib)?;
        Ok((lib, instance))
    }
}
