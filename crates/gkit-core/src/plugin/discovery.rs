use std::path::{Path, PathBuf};

use crate::plugin::error::{PluginError, PluginResult};

// ============================================================================
// DiscoveredPlugin
// ============================================================================

/// A plugin found on disk, with its name extracted from the filename
/// and the full path to the library file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPlugin {
    /// Plugin name extracted from the filename (e.g. "test" from "libgkit_plugin_test.dylib").
    pub name: String,
    /// Full path to the plugin library file.
    pub path: PathBuf,
}

// ============================================================================
// PluginSearchPath
// ============================================================================

/// Where to search for plugin library files.
#[derive(Debug, Clone)]
pub enum PluginSearchPath {
    /// A specific directory on the filesystem.
    Directory(PathBuf),
    /// Read the directory path from an environment variable (e.g. `GKIT_PLUGIN_PATH`).
    EnvVar(&'static str),
    /// Search in Cargo's target directory (`target/debug` and `target/release`).
    CargoTargetDir,
    /// A path relative to the current executable's location.
    RelativeToExe(&'static str),
}

impl PluginSearchPath {
    /// Resolve this search path to one or more concrete directory paths.
    ///
    /// Returns a `Vec` because some variants (e.g. `CargoTargetDir`) produce multiple
    /// candidate directories.
    pub fn resolve(&self) -> PluginResult<Vec<PathBuf>> {
        match self {
            PluginSearchPath::Directory(path) => {
                Ok(vec![path.clone()])
            }
            PluginSearchPath::EnvVar(var) => {
                let val = std::env::var(var).map_err(|_| PluginError::DirectoryNotFound {
                    path: PathBuf::from(format!("${}", var)),
                })?;
                Ok(vec![PathBuf::from(val)])
            }
            PluginSearchPath::CargoTargetDir => {
                let manifest_dir =
                    std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
                        PluginError::DirectoryNotFound {
                            path: PathBuf::from("CARGO_MANIFEST_DIR"),
                        }
                    })?;
                let base = PathBuf::from(&manifest_dir);
                Ok(vec![
                    base.join("target").join("debug"),
                    base.join("target").join("release"),
                ])
            }
            PluginSearchPath::RelativeToExe(relative) => {
                let exe_dir = std::env::current_exe()
                    .map_err(|e| PluginError::DirectoryNotFound {
                        path: PathBuf::from(format!("current_exe: {}", e)),
                    })?
                    .parent()
                    .ok_or_else(|| PluginError::DirectoryNotFound {
                        path: PathBuf::from("exe has no parent directory"),
                    })?
                    .to_path_buf();
                Ok(vec![exe_dir.join(relative)])
            }
        }
    }
}

// ============================================================================
// PluginDiscovery
// ============================================================================

/// Discovers plugin library files on the filesystem.
pub struct PluginDiscovery;

impl PluginDiscovery {
    /// Discover plugins across multiple search paths.
    ///
    /// Each `PluginSearchPath` is resolved to one or more directories, then each
    /// directory is scanned for plugin files matching the naming convention.
    pub fn discover(search_paths: &[PluginSearchPath]) -> PluginResult<Vec<DiscoveredPlugin>> {
        let mut all = Vec::new();
        for sp in search_paths {
            let dirs = sp.resolve()?;
            for dir in &dirs {
                let found = Self::scan(dir)?;
                all.extend(found);
            }
        }
        Ok(all)
    }

    /// Scan a single directory for plugin files.
    ///
    /// Returns an empty `Vec` if the directory does not exist, is not a directory,
    /// or contains no files matching the plugin naming convention.
    pub fn scan(dir: &Path) -> PluginResult<Vec<DiscoveredPlugin>> {
        if !dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut plugins = Vec::new();
        let entries = std::fs::read_dir(dir).map_err(|e| {
            let source: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
            PluginError::LoadFailed {
                path: dir.to_path_buf(),
                source,
            }
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                let source: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
                PluginError::LoadFailed {
                    path: dir.to_path_buf(),
                    source,
                }
            })?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = Self::extract_plugin_name(&path) {
                    plugins.push(DiscoveredPlugin { name, path });
                }
            }
        }

        Ok(plugins)
    }

    /// Extract the plugin name from a filename following the naming convention:
    ///
    /// - macOS/Linux: `libgkit_plugin_{name}.dylib` / `.so`
    /// - Windows: `gkit_plugin_{name}.dll`
    ///
    /// Returns `None` if the file does not match the convention.
    fn extract_plugin_name(path: &Path) -> Option<String> {
        let stem = path.file_stem()?.to_str()?;

        // Strip the platform-specific prefix
        let name = if let Some(suffix) = stem.strip_prefix("libgkit_plugin_") {
            suffix
        } else if let Some(suffix) = stem.strip_prefix("gkit_plugin_") {
            suffix
        } else {
            return None;
        };

        // Ensure the name is non-empty and the extension matches
        if name.is_empty() {
            return None;
        }

        let ext = path.extension()?.to_str()?;
        if !matches!(ext, "dylib" | "so" | "dll") {
            return None;
        }

        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_name_from_macos_dylib() {
        let path = Path::new("libgkit_plugin_test.dylib");
        assert_eq!(
            PluginDiscovery::extract_plugin_name(path),
            Some("test".to_string())
        );
    }

    #[test]
    fn extract_name_from_linux_so() {
        let path = Path::new("libgkit_plugin_my_plugin.so");
        assert_eq!(
            PluginDiscovery::extract_plugin_name(path),
            Some("my_plugin".to_string())
        );
    }

    #[test]
    fn extract_name_from_windows_dll() {
        let path = Path::new("gkit_plugin_test.dll");
        assert_eq!(
            PluginDiscovery::extract_plugin_name(path),
            Some("test".to_string())
        );
    }

    #[test]
    fn extract_rejects_non_plugin_prefix() {
        let path = Path::new("librandom.dylib");
        assert_eq!(PluginDiscovery::extract_plugin_name(path), None);
    }

    #[test]
    fn extract_rejects_prefix_without_name() {
        let path = Path::new("libgkit_plugin_.dylib");
        assert_eq!(PluginDiscovery::extract_plugin_name(path), None);
    }

    #[test]
    fn extract_rejects_readme_txt() {
        let path = Path::new("readme.txt");
        assert_eq!(PluginDiscovery::extract_plugin_name(path), None);
    }

    #[test]
    fn extract_rejects_wrong_extension() {
        let path = Path::new("libgkit_plugin_test.pdf");
        assert_eq!(PluginDiscovery::extract_plugin_name(path), None);
    }
}
