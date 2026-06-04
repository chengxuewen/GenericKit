//! UniFFI bindings for gkit-core — exports version info to Kotlin, Swift, Python, Ruby.

uniffi::include_scaffolding!("gkit_core");

/// Returns the full version string, e.g. "GenericKit 0.1.2 (x86_64-macos, debug, rustc ...)"
pub fn get_version_string() -> String {
    gkit_core::version::version_string()
}

/// Returns the target architecture, e.g. "x86_64", "aarch64"
pub fn get_target_arch() -> String {
    gkit_core::version::version_info().target_arch.clone()
}

/// Returns the target OS, e.g. "linux", "macos", "windows"
pub fn get_target_os() -> String {
    gkit_core::version::version_info().target_os.clone()
}
