mod adapt;

use gkit_media::protocols::rtc::peer::PeerConnectionFactory;
use stabby::string::String as StabbyString;
use std::ffi::c_void;

use adapt::LiveKitRsFactory;

#[stabby::export(canaries)]
pub extern "C" fn gkit_plugin_abi_version() -> u32 {
    1
}

#[stabby::export]
pub extern "C" fn gkit_plugin_backend_name() -> StabbyString {
    "libwebrtc".into()
}

#[unsafe(no_mangle)]
pub extern "C" fn gkit_plugin_create_factory() -> *mut c_void {
    let f: Box<dyn PeerConnectionFactory> = Box::new(LiveKitRsFactory::new());
    Box::into_raw(Box::new(f)) as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn gkit_plugin_destroy_factory(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut Box<dyn PeerConnectionFactory>);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gkit_plugin_factory_create_pc(factory_ptr: *mut c_void) -> *mut c_void {
    if factory_ptr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let f: &Box<dyn PeerConnectionFactory> =
            &*(factory_ptr as *const Box<dyn PeerConnectionFactory>);
        match f.create_peer_connection() {
            Ok(pc) => Box::into_raw(Box::new(pc)) as *mut c_void,
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gkit_plugin_destroy_pc(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(
                ptr as *mut Box<dyn gkit_media::protocols::rtc::peer::PeerConnection>,
            );
        }
    }
}
