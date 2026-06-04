//! UniFFI bindings for gkit-media — exports media API to Kotlin, Swift, Python, Ruby.

uniffi::include_scaffolding!("gkit_media");

/// Returns the full version string, e.g. "GenericKit 0.1.2 (x86_64-macos, debug, rustc ...)"
pub fn get_version_string() -> String {
    gkit_core::version::version_string()
}

/// Calls gkit_media::media_hello() and returns a greeting string.
pub fn hello() -> String {
    gkit_media::media_hello();
    "media_hello!".to_string()
}
