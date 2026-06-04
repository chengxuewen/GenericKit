//! WASM bindings for gkit-media — exports media API to JavaScript via wasm-bindgen.

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    /// Returns the full version string, e.g. "GenericKit 0.1.2 (x86_64-macos, debug, rustc ...)"
    #[wasm_bindgen]
    pub fn get_version() -> String {
        gkit_core::version::version_string()
    }

    /// Calls gkit_media::media_hello() and returns a greeting string.
    #[wasm_bindgen]
    pub fn media_hello() -> String {
        gkit_media::media_hello();
        "media_hello!".to_string()
    }
}
