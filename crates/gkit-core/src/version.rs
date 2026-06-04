/// Version information APIs — universally useful for all language bindings.
use std::sync::OnceLock;

static VERSION_INFO: OnceLock<VersionInfo> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub target_arch: String,
    pub target_os: String,
    pub rustc_version: String,
    pub build_profile: String,
}

impl VersionInfo {
    fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            target_arch: std::env::consts::ARCH.to_string(),
            target_os: std::env::consts::OS.to_string(),
            rustc_version: env!("GKIT_RUSTC_VERSION", "unknown").to_string(),
            build_profile: if cfg!(debug_assertions) {
                "debug".to_string()
            } else {
                "release".to_string()
            },
        }
    }
}

pub fn version_info() -> &'static VersionInfo {
    VERSION_INFO.get_or_init(VersionInfo::new)
}

pub fn version_string() -> String {
    let v = version_info();
    format!(
        "GenericKit {version} ({arch}-{os}, {profile}, rustc {rustc})",
        version = v.version,
        arch = v.target_arch,
        os = v.target_os,
        profile = v.build_profile,
        rustc = v.rustc_version,
    )
}
