use std::ffi::c_void;
use std::path::PathBuf;

use gkit_core::plugin::loader::PluginLib;

type CreateFactoryFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);

fn plugin_dylib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().unwrap().parent().unwrap();
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let stem = if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        "libgkit_plugin_webrtc_libwebrtc"
    } else {
        "gkit_plugin_webrtc_libwebrtc"
    };
    let ext = if cfg!(target_os = "macos") { "dylib" }
        else if cfg!(target_os = "linux") { "so" }
        else { "dll" };
    workspace_root.join("target").join(profile).join(format!("{stem}.{ext}"))
}

fn load_lib() -> PluginLib {
    let path = plugin_dylib_path();
    assert!(path.exists(), "plugin dylib not found at {path:?}");
    unsafe { PluginLib::open(&path).expect("failed to load plugin") }
}

#[test]
fn load_plugin_and_check_abi_version() {
    let lib = load_lib();
    unsafe { lib.check_abi_version().expect("ABI version mismatch") };
}

#[test]
fn load_plugin_and_call_backend_name() {
    let lib = load_lib();
    let name_fn = unsafe {
        lib.get_stabbied::<extern "C" fn() -> stabby::string::String>(b"gkit_plugin_backend_name")
    }.expect("symbol not found");
    assert_eq!(name_fn().as_str(), "libwebrtc");
}

#[test]
fn create_and_destroy_factory() {
    let lib = load_lib();
    let create: libloading::Symbol<CreateFactoryFn> =
        unsafe { lib.get_symbol(b"gkit_plugin_create_factory") }.unwrap();
    let destroy: libloading::Symbol<DestroyFn> =
        unsafe { lib.get_symbol(b"gkit_plugin_destroy_factory") }.unwrap();
    let ptr = unsafe { create() };
    assert!(!ptr.is_null());
    unsafe { destroy(ptr) };
}

#[test]
#[ignore = "libwebrtc PeerConnectionFactory init requires platform setup (ObjC on macOS)"]
fn create_peer_connection_from_plugin() {
    // This test works when run in the proper macOS app context
    // (e.g., the webrtc_loopback example which initializes the ObjC runloop).
    let lib = load_lib();
    let create_factory: libloading::Symbol<CreateFactoryFn> =
        unsafe { lib.get_symbol(b"gkit_plugin_create_factory") }.unwrap();
    let destroy_factory: libloading::Symbol<DestroyFn> =
        unsafe { lib.get_symbol(b"gkit_plugin_destroy_factory") }.unwrap();
    let factory_ptr = unsafe { create_factory() };
    assert!(!factory_ptr.is_null());
    unsafe { destroy_factory(factory_ptr) };
}

#[test]
fn rtc_engine_loads_libwebrtc_plugin() {
    use gkit_media::protocols::rtc::peer::engine::RtcEngine;

    let loaded = RtcEngine::load_plugins();
    assert!(loaded > 0, "no plugins loaded");

    let factory = RtcEngine::create("libwebrtc");
    assert!(factory.is_ok(), "libwebrtc backend not found: {:?}", factory.err());
}

#[test]
#[ignore = "libwebrtc PeerConnectionFactory init requires platform setup"]
fn rtc_engine_creates_peer_connection_from_plugin() {
    use gkit_media::protocols::rtc::peer::engine::RtcEngine;
    use gkit_media::protocols::rtc::peer::core::ConnectionState;

    RtcEngine::load_plugins();
    let factory = RtcEngine::create("libwebrtc").expect("libwebrtc not loaded");
    let pc = factory.create_peer_connection().expect("create PC failed");
    assert_eq!(pc.connection_state(), ConnectionState::New);
}
