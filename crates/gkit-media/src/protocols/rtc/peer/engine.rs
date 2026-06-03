use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{OnceLock, RwLock};

use gkit_core::plugin::discovery::{PluginDiscovery, PluginSearchPath};
use gkit_core::plugin::loader::PluginLib;

use crate::plugin::registry::PluginRegistry;
use crate::protocols::rtc::peer::core::{MediaError, MediaResult, PeerConnectionFactory};

type FactoryCreator = fn() -> Box<dyn PeerConnectionFactory>;

fn registry() -> &'static RwLock<HashMap<&'static str, FactoryCreator>> {
    static REG: OnceLock<RwLock<HashMap<&'static str, FactoryCreator>>> = OnceLock::new();
    REG.get_or_init(|| RwLock::new(HashMap::new()))
}

fn plugin_registry() -> &'static PluginRegistry<Box<dyn PeerConnectionFactory>> {
    static PREG: OnceLock<PluginRegistry<Box<dyn PeerConnectionFactory>>> = OnceLock::new();
    PREG.get_or_init(|| {
        let reg = PluginRegistry::new();
        reg.set_default_order(vec!["libwebrtc".into(), "webrtc-rs".into()]);
        reg
    })
}

pub struct RtcEngine;

impl RtcEngine {
    fn ensure_plugins_loaded() {
        #[cfg(not(test))]
        {
            static LOADED: OnceLock<()> = OnceLock::new();
            LOADED.get_or_init(|| { Self::load_plugins(); });
        }
    }

    pub fn create(backend_name: &str) -> MediaResult<Box<dyn PeerConnectionFactory>> {
        Self::ensure_plugins_loaded();
        if let Ok(factory) = plugin_registry().create(Some(backend_name)) {
            return Ok(factory);
        }
        let map = registry().read().unwrap();
        let creator = map.get(backend_name)
            .ok_or_else(|| MediaError::new(format!("unknown RTC backend: {backend_name}")))?;
        Ok(creator())
    }

    pub fn register(name: &'static str, creator: FactoryCreator) {
        registry().write().unwrap().entry(name).or_insert(creator);
    }

    pub fn registered_types() -> Vec<String> {
        let mut names = registry().read().unwrap().keys().map(|k| k.to_string()).collect::<Vec<_>>();
        for name in plugin_registry().names() {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }

    pub fn create_default() -> MediaResult<Box<dyn PeerConnectionFactory>> {
        let names = Self::registered_types();
        for preferred in &["webrtc-rs", "libwebrtc", "wasm"] {
            if names.iter().any(|n| n == preferred) {
                return Self::create(preferred);
            }
        }
        if let Ok(factory) = plugin_registry().create(None) {
            return Ok(factory);
        }
        names.first()
            .ok_or_else(|| MediaError::new("no RTC backend registered"))
            .and_then(|n| Self::create(n))
    }

    pub fn create_for_platform() -> MediaResult<Box<dyn PeerConnectionFactory>> {
        #[cfg(target_os = "linux")]
        if std::path::Path::new("/etc/nv_tegra_release").exists() {
            if let Ok(f) = Self::create("libwebrtc") { return Ok(f); }
        }
        Self::create_default()
    }

    pub fn load_plugins() -> usize {
        static LOADED: OnceLock<usize> = OnceLock::new();
        *LOADED.get_or_init(|| {
            let mut search_paths = vec![
                PluginSearchPath::RelativeToExe("../plugins"),
                PluginSearchPath::RelativeToExe(".."),
                PluginSearchPath::CargoTargetDir,
            ];
            if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
                let p = std::path::PathBuf::from(&manifest);
                if let Some(ws) = p.parent().and_then(|x| x.parent()) {
                    search_paths.push(PluginSearchPath::Directory(ws.join("build/plugins/webrtc")));
                    search_paths.push(PluginSearchPath::Directory(ws.join("target/debug")));
                    search_paths.push(PluginSearchPath::Directory(ws.join("target/release")));
                }
            }
            let discovered = match PluginDiscovery::discover(&search_paths) { Ok(p) => p, Err(_) => return 0 };
            let mut loaded = 0;
            for p in &discovered {
                if let Err(_) = Self::try_load_plugin(p) { continue; }
                loaded += 1;
            }
            loaded
        })
    }

    fn try_load_plugin(plugin: &gkit_core::plugin::discovery::DiscoveredPlugin) -> MediaResult<()> {
        let lib = unsafe { PluginLib::open(&plugin.path) }
            .map_err(|e| MediaError::new(format!("dlopen {}: {e}", plugin.path.display())))?;
        unsafe { lib.check_abi_version() }
            .map_err(|e| MediaError::new(format!("ABI mismatch {}: {e}", plugin.name)))?;
        let name_fn_sym = unsafe { lib.get_stabbied::<extern "C" fn() -> stabby::string::String>(b"gkit_plugin_backend_name") }
            .map_err(|e| MediaError::new(format!("missing backend_name in {}: {e}", plugin.name)))?;
        let backend_name = name_fn_sym().to_string();
        let _name_fn: extern "C" fn() -> stabby::string::String = *name_fn_sym;
        drop(name_fn_sym);
        let create_sym = unsafe { lib.get_symbol::<unsafe extern "C" fn() -> *mut c_void>(b"gkit_plugin_create_factory") }
            .map_err(|e| MediaError::new(format!("missing create_factory in {}: {e}", plugin.name)))?;
        let create_fn: unsafe extern "C" fn() -> *mut c_void = *create_sym;
        drop(create_sym);
        let lib_arc = std::sync::Arc::new(lib);
        let factory_fn = Box::new(move || {
            let _keep = &lib_arc;
            let ptr = unsafe { create_fn() };
            assert!(!ptr.is_null());
            unsafe { *Box::from_raw(ptr as *mut Box<dyn PeerConnectionFactory>) }
        });
        plugin_registry().register(&backend_name, factory_fn);
        Ok(())
    }
}
