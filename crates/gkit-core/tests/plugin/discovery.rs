use std::fs;
use std::path::PathBuf;

use gkit_core::plugin::discovery::{PluginDiscovery, PluginSearchPath};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "gkit_test_discovery_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unknown")
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn scan_empty_directory_returns_none() {
    let dir = TempDir::new();
    let plugins = PluginDiscovery::scan(dir.path()).unwrap();
    assert!(plugins.is_empty());
}

#[test]
fn scan_finds_dylib_plugins() {
    let dir = TempDir::new();
    let plugin_path = dir.path().join("libgkit_plugin_test.dylib");
    fs::write(&plugin_path, b"mock dylib").unwrap();

    let plugins = PluginDiscovery::scan(dir.path()).unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].name, "test");
}

#[test]
fn scan_ignores_non_plugin_files() {
    let dir = TempDir::new();
    fs::write(dir.path().join("readme.txt"), b"hello").unwrap();
    fs::write(dir.path().join("librandom.dylib"), b"not gkit").unwrap();

    let plugins = PluginDiscovery::scan(dir.path()).unwrap();
    assert!(plugins.is_empty());
}

#[test]
fn plugin_search_path_resolves_cargo_target_dir() {
    let path = PluginSearchPath::CargoTargetDir;
    let dirs = path.resolve().unwrap();
    assert!(!dirs.is_empty());
}

#[test]
fn plugin_search_path_env_var_fallback() {
    let old_val = std::env::var("GKIT_PLUGIN_PATH").ok();
    // SAFETY: Single-threaded test context; env var restore prevents cross-test leakage.
    unsafe { std::env::set_var("GKIT_PLUGIN_PATH", "/tmp/nonexistent") };

    let path = PluginSearchPath::EnvVar("GKIT_PLUGIN_PATH");
    let dirs = path.resolve().unwrap();
    assert_eq!(dirs.len(), 1);

    if let Some(val) = old_val {
        // SAFETY: Restoring original value in single-threaded test context.
        unsafe { std::env::set_var("GKIT_PLUGIN_PATH", val) };
    } else {
        // SAFETY: Removing test env var in single-threaded test context.
        unsafe { std::env::remove_var("GKIT_PLUGIN_PATH") };
    }
}
