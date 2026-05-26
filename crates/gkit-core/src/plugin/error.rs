use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    #[error("failed to load {path}: {source}")]
    LoadFailed {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("ABI mismatch: plugin={plugin} host={host}")]
    AbiVersionMismatch { plugin: u32, host: u32 },

    #[error("missing symbol '{symbol}' in '{name}'")]
    MissingSymbol { name: String, symbol: String },
}

pub type PluginResult<T> = Result<T, PluginError>;
