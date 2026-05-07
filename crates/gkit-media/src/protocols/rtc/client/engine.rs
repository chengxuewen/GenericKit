use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use super::core::{MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration};

type FactoryCreator = fn() -> Box<dyn PeerConnectionFactory>;

fn registry() -> &'static RwLock<HashMap<&'static str, FactoryCreator>> {
    static REG: OnceLock<RwLock<HashMap<&'static str, FactoryCreator>>> = OnceLock::new();
    REG.get_or_init(|| RwLock::new(HashMap::new()))
}

pub struct RtcEngine;

impl RtcEngine {
    pub fn create(backend_name: &str) -> MediaResult<Box<dyn PeerConnectionFactory>> {
        let map = registry().read().unwrap();
        let creator = map
            .get(backend_name)
            .ok_or_else(|| MediaError::new(format!("unknown RTC backend: {backend_name}")))?;
        creator()
    }

    pub fn register(name: &'static str, creator: FactoryCreator) {
        registry().write().unwrap().entry(name).or_insert(creator);
    }

    pub fn registered_types() -> Vec<String> {
        registry()
            .read()
            .unwrap()
            .keys()
            .map(|k| k.to_string())
            .collect()
    }

    pub fn create_default() -> MediaResult<Box<dyn PeerConnectionFactory>> {
        let names = Self::registered_types();
        for preferred in &["webrtc-rs", "google_lk", "wasm"] {
            if names.iter().any(|n| n == preferred) {
                return Self::create(preferred);
            }
        }
        names
            .first()
            .ok_or_else(|| MediaError::new("no RTC backend registered"))
            .and_then(|n| Self::create(n))
    }

    pub fn create_for_platform() -> MediaResult<Box<dyn PeerConnectionFactory>> {
        #[cfg(target_os = "linux")]
        if std::path::Path::new("/etc/nv_tegra_release").exists() {
            if let Ok(f) = Self::create("google_lk") {
                return Ok(f);
            }
        }
        Self::create_default()
    }
}
