//! WASM bindings for gkit-core — exports version info to JavaScript via wasm-bindgen.

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    /// Returns the full version string, e.g. "GenericKit 0.1.2 (x86_64-macos, debug, rustc ...)"
    #[wasm_bindgen]
    pub fn get_version() -> String {
        gkit_core::version::version_string()
    }

    /// Returns the target architecture, e.g. "x86_64", "aarch64"
    #[wasm_bindgen]
    pub fn get_target_arch() -> String {
        gkit_core::version::version_info().target_arch.clone()
    }

    /// Returns the target OS, e.g. "linux", "macos", "windows"
    #[wasm_bindgen]
    pub fn get_target_os() -> String {
        gkit_core::version::version_info().target_os.clone()
    }
}
