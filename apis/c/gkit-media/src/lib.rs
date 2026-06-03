use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use gkit_media;
use gkit_media::protocols::rtc::peer::core::{
    DataChannel as DcTrait, PeerConnection as PcTrait, SessionDescription, PeerConnectionFactory,
};
use gkit_media::protocols::rtc::peer::engine::RtcEngine;

// --- SCTP settings (global, set before creating connections) ---

#[repr(C)]
pub struct RtcSctpSettings {
    pub recv_buffer_size: i32,
    pub send_buffer_size: i32,
    pub max_chunks_on_queue: i32,
    pub initial_congestion_window: i32,
    pub max_burst: i32,
    pub congestion_control_module: i32,
    pub delayed_sack_time_ms: i32,
    pub min_retransmit_timeout_ms: i32,
    pub max_retransmit_timeout_ms: i32,
    pub initial_retransmit_timeout_ms: i32,
    pub max_retransmit_attempts: i32,
    pub heartbeat_interval_ms: i32,
}

static mut SCTP_SETTINGS: RtcSctpSettings = RtcSctpSettings {
    recv_buffer_size: 0, send_buffer_size: 0, max_chunks_on_queue: 0,
    initial_congestion_window: 0, max_burst: 0, congestion_control_module: 0,
    delayed_sack_time_ms: 0, min_retransmit_timeout_ms: 0, max_retransmit_timeout_ms: 0,
    initial_retransmit_timeout_ms: 0, max_retransmit_attempts: 0, heartbeat_interval_ms: 0,
};

// --- C callback types ---

pub type PcStateCallback = Option<unsafe extern "C" fn(pc: *mut std::ffi::c_void, state: i32, user_data: *mut std::ffi::c_void)>;
pub type PcDescriptionCallback = Option<unsafe extern "C" fn(pc: *mut std::ffi::c_void, sdp: *const c_char, sdp_type: *const c_char, user_data: *mut std::ffi::c_void)>;
pub type PcCandidateCallback = Option<unsafe extern "C" fn(pc: *mut std::ffi::c_void, cand: *const c_char, mid: *const c_char, user_data: *mut std::ffi::c_void)>;
pub type PcDataChannelCallback = Option<unsafe extern "C" fn(pc: *mut std::ffi::c_void, dc: *mut std::ffi::c_void, user_data: *mut std::ffi::c_void)>;
pub type DcMessageCallback = Option<unsafe extern "C" fn(dc: *mut std::ffi::c_void, data: *const u8, len: i32, user_data: *mut std::ffi::c_void)>;
pub type VoidCallback = Option<unsafe extern "C" fn(handle: *mut std::ffi::c_void, user_data: *mut std::ffi::c_void)>;

#[allow(dead_code)]
struct PcCallbackData {
    #[allow(dead_code)]
    user_data: *mut std::ffi::c_void,
    #[allow(dead_code)]
    on_state_change: PcStateCallback,
    on_ice_state_change: PcStateCallback,
    on_gathering_state_change: PcStateCallback,
    on_signaling_state_change: PcStateCallback,
    on_local_description: PcDescriptionCallback,
    on_local_candidate: PcCandidateCallback,
    on_data_channel: PcDataChannelCallback,
}

#[allow(dead_code)]
struct DcCallbackData {
    user_data: *mut std::ffi::c_void,
    on_open: VoidCallback,
    on_closed: VoidCallback,
    on_error: Option<unsafe extern "C" fn(dc: *mut std::ffi::c_void, err: *const c_char, user_data: *mut std::ffi::c_void)>,
    on_message: DcMessageCallback,
}

// --- Opaque handle wrappers ---

struct PcHandleBox {
    inner: Box<dyn PcTrait>,
    callbacks: PcCallbackData,
}

struct DcHandleBox {
    inner: Box<dyn DcTrait>,
    callbacks: DcCallbackData,
}

fn pc_ptr_to_inner<'a>(ptr: *mut std::ffi::c_void) -> Option<&'a mut Box<dyn PcTrait>> {
    if ptr.is_null() { None }
    else { unsafe { Some(&mut (*(ptr as *mut PcHandleBox)).inner) } }
}

fn dc_ptr_to_inner<'a>(ptr: *mut std::ffi::c_void) -> Option<&'a mut Box<dyn DcTrait>> {
    if ptr.is_null() { None }
    else { unsafe { Some(&mut (*(ptr as *mut DcHandleBox)).inner) } }
}

fn pc_ptr_to_handle<'a>(ptr: *mut std::ffi::c_void) -> Option<&'a mut PcHandleBox> {
    if ptr.is_null() { None }
    else { unsafe { Some(&mut *(ptr as *mut PcHandleBox)) } }
}

fn dc_ptr_to_handle<'a>(ptr: *mut std::ffi::c_void) -> Option<&'a mut DcHandleBox> {
    if ptr.is_null() { None }
    else { unsafe { Some(&mut *(ptr as *mut DcHandleBox)) } }
}

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

// --- existing ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_hello() {
    gkit_media::media_hello();
}

// ============================================================================
// Factory (create via backend name, then create PeerConnection)
// ============================================================================

struct FactoryHandleBox {
    inner: Box<dyn PeerConnectionFactory>,
}

fn factory_ptr_to_inner<'a>(ptr: *mut std::ffi::c_void) -> Option<&'a mut Box<dyn PeerConnectionFactory>> {
    if ptr.is_null() { None }
    else { unsafe { Some(&mut (*(ptr as *mut FactoryHandleBox)).inner) } }
}

/// Create an RTC factory by backend name. Returns opaque handle, or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_create_factory(
    backend_name: *const c_char,
) -> *mut std::ffi::c_void {
    unsafe {
        let name = CStr::from_ptr(backend_name).to_str().unwrap_or_default();
        match RtcEngine::create(name) {
            Ok(f) => Box::into_raw(Box::new(FactoryHandleBox { inner: f })) as *mut std::ffi::c_void,
            Err(_) => std::ptr::null_mut(),
        }
    }
}

/// Destroy a factory and free resources.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_destroy_factory(handle: *mut std::ffi::c_void) { unsafe {
    if handle.is_null() { return; }
    let _ = Box::from_raw(handle as *mut FactoryHandleBox);
}}

/// Get the backend name of a factory. Returns null if handle is invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_factory_backend_name(
    handle: *mut std::ffi::c_void,
) -> *mut c_char {
    let Some(f) = factory_ptr_to_inner(handle) else { return std::ptr::null_mut() };
    to_c_string(f.backend_name())
}

/// Create a PeerConnection from a factory handle. Returns opaque handle, or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_factory_create_peer_connection(
    factory: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    let Some(f) = factory_ptr_to_inner(factory) else { return std::ptr::null_mut() };
    match f.create_peer_connection() {
        Ok(inner) => Box::into_raw(Box::new(PcHandleBox {
            inner,
            callbacks: PcCallbackData {
                user_data: std::ptr::null_mut(),
                on_state_change: None,
                on_ice_state_change: None,
                on_gathering_state_change: None,
                on_signaling_state_change: None,
                on_local_description: None,
                on_local_candidate: None,
                on_data_channel: None,
            },
        })) as *mut std::ffi::c_void,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get the list of registered backend names.
/// Returns count; *out_names receives an array of strings. Caller must free with
/// gkit_media_rtc_free_string_array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_get_registered_backends(
    out_count: *mut i32,
) -> *mut *mut c_char {
    let names = RtcEngine::registered_types();
    if !out_count.is_null() {
        unsafe { *out_count = names.len() as i32; }
    }
    if names.is_empty() {
        return std::ptr::null_mut();
    }
    let mut ptrs: Vec<*mut c_char> = names.iter().map(|s| to_c_string(s)).collect();
    let arr = ptrs.as_mut_ptr();
    std::mem::forget(ptrs);
    arr
}

/// Free a string array returned by gkit_media_rtc_get_registered_backends.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_free_string_array(
    arr: *mut *mut c_char,
    count: i32,
) { unsafe {
    if arr.is_null() { return; }
    for i in 0..count as isize {
        let s = *arr.offset(i);
        if !s.is_null() {
            let _ = CString::from_raw(s);
        }
    }
    let _ = Vec::from_raw_parts(arr, count as usize, count as usize);
}}

// ============================================================================
// PeerConnection
// ============================================================================

/// Create a PeerConnection. Returns opaque handle, or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_create_peer_connection() -> *mut std::ffi::c_void {
    let inner = gkit_media::make_peer_connection();
    Box::into_raw(Box::new(PcHandleBox {
        inner,
        callbacks: PcCallbackData {
            user_data: std::ptr::null_mut(),
            on_state_change: None,
            on_ice_state_change: None,
            on_gathering_state_change: None,
            on_signaling_state_change: None,
            on_local_description: None,
            on_local_candidate: None,
            on_data_channel: None,
        },
    })) as *mut std::ffi::c_void
}

/// Destroy a PeerConnection and free resources.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_destroy_peer_connection(handle: *mut std::ffi::c_void) { unsafe {
    if handle.is_null() { return; }
    let _ = Box::from_raw(handle as *mut PcHandleBox);
}}

/// Create an SDP offer. Returns 0 on success; SDP written to *out_sdp.
/// Caller must free *out_sdp with gkit_media_rtc_free_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_create_offer(
    handle: *mut std::ffi::c_void,
    out_sdp: *mut *mut c_char,
) -> i32 {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.create_offer() {
        Ok(desc) => {
            let s = format!("{}\n{}", desc.sdp_type, desc.sdp);
            if !out_sdp.is_null() {
                unsafe { *out_sdp = to_c_string(&s); }
            }
            0
        }
        Err(_) => -1,
    }
}

/// Create an SDP answer. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_create_answer(
    handle: *mut std::ffi::c_void,
    out_sdp: *mut *mut c_char,
) -> i32 {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.create_answer() {
        Ok(desc) => {
            let s = format!("{}\n{}", desc.sdp_type, desc.sdp);
            if !out_sdp.is_null() {
                unsafe { *out_sdp = to_c_string(&s); }
            }
            0
        }
        Err(_) => -1,
    }
}

/// Set local SDP description. SDP format: "type\nsdp".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_local_description(
    handle: *mut std::ffi::c_void,
    sdp: *const c_char,
) -> i32 { unsafe {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    if sdp.is_null() {
        return -1;
    }
    let Ok(s) = CStr::from_ptr(sdp).to_str() else { return -1 };
    let Some((sdp_type, sdp_body)) = s.split_once('\n') else { return -1 };
    let desc = SessionDescription {
        sdp_type: sdp_type.to_string(),
        sdp: sdp_body.to_string(),
    };
    match pc.set_local_description(&desc) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}}

/// Set remote SDP description.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_remote_description(
    handle: *mut std::ffi::c_void,
    sdp: *const c_char,
) -> i32 { unsafe {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    if sdp.is_null() {
        return -1;
    }
    let Ok(s) = CStr::from_ptr(sdp).to_str() else { return -1 };
    let Some((sdp_type, sdp_body)) = s.split_once('\n') else { return -1 };
    let desc = SessionDescription {
        sdp_type: sdp_type.to_string(),
        sdp: sdp_body.to_string(),
    };
    match pc.set_remote_description(&desc) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}}

/// Add an ICE candidate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_add_ice_candidate(
    handle: *mut std::ffi::c_void,
    candidate: *const c_char,
    sdp_mid: *const c_char,
) -> i32 { unsafe {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    if candidate.is_null() || sdp_mid.is_null() {
        return -1;
    }
    let c = CStr::from_ptr(candidate).to_str().unwrap_or("");
    let m = CStr::from_ptr(sdp_mid).to_str().unwrap_or("");
    match pc.add_ice_candidate(c, m) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}}

/// Create a DataChannel on this PeerConnection.
/// Returns opaque DataChannel handle or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_create_data_channel(
    handle: *mut std::ffi::c_void,
    label: *const c_char,
) -> *mut std::ffi::c_void { unsafe {
    let Some(pc) = pc_ptr_to_inner(handle) else { return std::ptr::null_mut() };
    if label.is_null() {
        return std::ptr::null_mut();
    }
    let l = CStr::from_ptr(label).to_str().unwrap_or("");
    match pc.create_data_channel(l) {
        Ok(dc) => Box::into_raw(Box::new(DcHandleBox {
            inner: dc,
            callbacks: DcCallbackData {
                user_data: std::ptr::null_mut(),
                on_open: None,
                on_closed: None,
                on_error: None,
                on_message: None,
            },
        })) as *mut std::ffi::c_void,
        Err(_) => std::ptr::null_mut(),
    }
}}

/// Get ICE connection state. Returns 0=New, 1=Checking, 2=Connected,
/// 3=Completed, 4=Failed, 5=Disconnected, 6=Closed, -1=error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_ice_state(
    handle: *mut std::ffi::c_void,
) -> i32 {
    use gkit_media::protocols::rtc::peer::core::IceConnectionState;
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.ice_connection_state() {
        IceConnectionState::New => 0,
        IceConnectionState::Checking => 1,
        IceConnectionState::Connected => 2,
        IceConnectionState::Completed => 3,
        IceConnectionState::Failed => 4,
        IceConnectionState::Disconnected => 5,
        IceConnectionState::Closed => 6,
    }
}

/// Get connection state. Returns 0=New, 1=Connecting, 2=Connected,
/// 3=Disconnected, 4=Failed, 5=Closed, -1=error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_connection_state(
    handle: *mut std::ffi::c_void,
) -> i32 {
    use gkit_media::protocols::rtc::peer::core::ConnectionState;
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.connection_state() {
        ConnectionState::New => 0,
        ConnectionState::Connecting => 1,
        ConnectionState::Connected => 2,
        ConnectionState::Disconnected => 3,
        ConnectionState::Failed => 4,
        ConnectionState::Closed => 5,
    }
}

/// Get ICE gathering state. Returns 0=New, 1=Gathering, 2=Complete, -1=error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_gathering_state(
    handle: *mut std::ffi::c_void,
) -> i32 {
    use gkit_media::protocols::rtc::peer::core::GatheringState;
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.gathering_state() {
        GatheringState::New => 0,
        GatheringState::Gathering => 1,
        GatheringState::Complete => 2,
    }
}

/// Get signaling state. Returns 0=Stable, 1=HaveLocalOffer, 2=HaveRemoteOffer,
/// 3=HaveLocalPranswer, 4=HaveRemotePranswer, -1=error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_signaling_state(
    handle: *mut std::ffi::c_void,
) -> i32 {
    use gkit_media::protocols::rtc::peer::core::SignalingState;
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.signaling_state() {
        SignalingState::Stable => 0,
        SignalingState::HaveLocalOffer => 1,
        SignalingState::HaveRemoteOffer => 2,
        SignalingState::HaveLocalPranswer => 3,
        SignalingState::HaveRemotePranswer => 4,
    }
}

/// Get local description. Returns 0 on success; SDP written to *out_sdp.
/// Caller must free *out_sdp with gkit_media_rtc_free_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_get_local_description(
    handle: *mut std::ffi::c_void,
    out_sdp: *mut *mut c_char,
) -> i32 {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.local_description() {
        Ok(desc) => {
            let s = format!("{}\n{}", desc.sdp_type, desc.sdp);
            if !out_sdp.is_null() {
                unsafe { *out_sdp = to_c_string(&s); }
            }
            0
        }
        Err(_) => -1,
    }
}

/// Get remote description. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_get_remote_description(
    handle: *mut std::ffi::c_void,
    out_sdp: *mut *mut c_char,
) -> i32 {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.remote_description() {
        Ok(desc) => {
            let s = format!("{}\n{}", desc.sdp_type, desc.sdp);
            if !out_sdp.is_null() {
                unsafe { *out_sdp = to_c_string(&s); }
            }
            0
        }
        Err(_) => -1,
    }
}

/// Close the PeerConnection.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_close(
    handle: *mut std::ffi::c_void,
) -> i32 {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    match pc.close() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

// ============================================================================
// DataChannel
// ============================================================================

/// Get DataChannel label. Caller must free with gkit_media_rtc_free_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_label(
    handle: *mut std::ffi::c_void,
) -> *mut c_char {
    let Some(dc) = dc_ptr_to_inner(handle) else { return std::ptr::null_mut() };
    to_c_string(dc.label())
}

/// Send text data. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_send_text(
    handle: *mut std::ffi::c_void,
    data: *const c_char,
) -> i32 { unsafe {
    let Some(dc) = dc_ptr_to_inner(handle) else { return -1 };
    if data.is_null() {
        return -1;
    }
    let s = CStr::from_ptr(data).to_str().unwrap_or("");
    match dc.send_text(s) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}}

/// Send binary data. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_send_bytes(
    handle: *mut std::ffi::c_void,
    data: *const u8,
    len: usize,
) -> i32 { unsafe {
    let Some(dc) = dc_ptr_to_inner(handle) else { return -1 };
    if data.is_null() {
        return -1;
    }
    let bytes = std::slice::from_raw_parts(data, len);
    match dc.send_bytes(bytes) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}}

/// Get DataChannel ready state. Returns 0=Connecting, 1=Open,
/// 2=Closing, 3=Closed, -1=error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_ready_state(
    handle: *mut std::ffi::c_void,
) -> i32 {
    use gkit_media::protocols::rtc::peer::core::DataChannelState;
    let Some(dc) = dc_ptr_to_inner(handle) else { return -1 };
    match dc.ready_state() {
        DataChannelState::Connecting => 0,
        DataChannelState::Open => 1,
        DataChannelState::Closing => 2,
        DataChannelState::Closed => 3,
    }
}

/// Close the DataChannel. Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_close(
    handle: *mut std::ffi::c_void,
) -> i32 {
    let Some(dc) = dc_ptr_to_inner(handle) else { return -1 };
    match dc.close() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Destroy a DataChannel and free resources.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_destroy_data_channel(handle: *mut std::ffi::c_void) { unsafe {
    if handle.is_null() {
        return;
    }
    let _ = Box::from_raw(handle as *mut DcHandleBox);
}}

/// Get DataChannel stream ID. Returns stream ID or -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_stream_id(
    handle: *mut std::ffi::c_void,
) -> i32 {
    let Some(dc) = dc_ptr_to_inner(handle) else { return -1 };
    dc.stream_id().map(|v| v as i32).unwrap_or(-1)
}

/// Get DataChannel protocol. Caller must free with gkit_media_rtc_free_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_protocol(
    handle: *mut std::ffi::c_void,
) -> *mut c_char {
    let Some(dc) = dc_ptr_to_inner(handle) else { return std::ptr::null_mut() };
    dc.protocol().map(|p| to_c_string(&p)).unwrap_or(std::ptr::null_mut())
}

/// Get local network address. Caller must free with gkit_media_rtc_free_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_get_local_address(
    handle: *mut std::ffi::c_void,
) -> *mut c_char {
    let Some(pc) = pc_ptr_to_inner(handle) else { return std::ptr::null_mut() };
    pc.local_address().map(|a| to_c_string(&a)).unwrap_or(std::ptr::null_mut())
}

/// Get remote network address. Caller must free with gkit_media_rtc_free_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_get_remote_address(
    handle: *mut std::ffi::c_void,
) -> *mut c_char {
    let Some(pc) = pc_ptr_to_inner(handle) else { return std::ptr::null_mut() };
    pc.remote_address().map(|a| to_c_string(&a)).unwrap_or(std::ptr::null_mut())
}

/// Get max DataChannel stream ID. Returns max stream ID or -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_get_max_data_channel_stream(
    handle: *mut std::ffi::c_void,
) -> i32 {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    pc.max_data_channel_stream().map(|v| v as i32).unwrap_or(-1)
}

/// Get remote max message size. Returns size in bytes or -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_get_remote_max_message_size(
    handle: *mut std::ffi::c_void,
) -> i32 {
    let Some(pc) = pc_ptr_to_inner(handle) else { return -1 };
    pc.remote_max_message_size().map(|v| v as i32).unwrap_or(-1)
}

// ============================================================================
// Utility
// ============================================================================

/// Free a string returned by any gkit_media_rtc_* function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_free_string(s: *mut c_char) { unsafe {
    if s.is_null() {
        return;
    }
    let _ = CString::from_raw(s);
}}

// ============================================================================
// SCTP Settings
// ============================================================================

/// Set global SCTP transport settings. Call before creating connections.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_set_sctp_settings(settings: *const RtcSctpSettings) {
    if settings.is_null() { return; }
    unsafe { SCTP_SETTINGS = std::ptr::read(settings); }
}

// ============================================================================
// Callbacks — PeerConnection
// ============================================================================

/// Set user pointer for any opaque handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_set_user_pointer(
    handle: *mut std::ffi::c_void,
    ptr: *mut std::ffi::c_void,
) {
    if handle.is_null() { return; }
    unsafe {
        let raw = handle as *mut PcHandleBox;
        if !raw.is_null() { (*raw).callbacks.user_data = ptr; }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_state_change_callback(
    handle: *mut std::ffi::c_void, cb: PcStateCallback) -> i32 {
    let Some(h) = pc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_state_change = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_ice_state_change_callback(
    handle: *mut std::ffi::c_void, cb: PcStateCallback) -> i32 {
    let Some(h) = pc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_ice_state_change = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_gathering_state_change_callback(
    handle: *mut std::ffi::c_void, cb: PcStateCallback) -> i32 {
    let Some(h) = pc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_gathering_state_change = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_signaling_state_change_callback(
    handle: *mut std::ffi::c_void, cb: PcStateCallback) -> i32 {
    let Some(h) = pc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_signaling_state_change = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_local_description_callback(
    handle: *mut std::ffi::c_void, cb: PcDescriptionCallback) -> i32 {
    let Some(h) = pc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_local_description = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_local_candidate_callback(
    handle: *mut std::ffi::c_void, cb: PcCandidateCallback) -> i32 {
    let Some(h) = pc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_local_candidate = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_peer_connection_set_data_channel_callback(
    handle: *mut std::ffi::c_void, cb: PcDataChannelCallback) -> i32 {
    let Some(h) = pc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_data_channel = cb; 0
}

// ============================================================================
// Callbacks — DataChannel
// ============================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_set_open_callback(
    handle: *mut std::ffi::c_void, cb: VoidCallback) -> i32 {
    let Some(h) = dc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_open = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_set_closed_callback(
    handle: *mut std::ffi::c_void, cb: VoidCallback) -> i32 {
    let Some(h) = dc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_closed = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_set_error_callback(
    handle: *mut std::ffi::c_void, cb: Option<unsafe extern "C" fn(dc: *mut std::ffi::c_void, err: *const c_char, user_data: *mut std::ffi::c_void)>) -> i32 {
    let Some(h) = dc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_error = cb; 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_data_channel_set_message_callback(
    handle: *mut std::ffi::c_void, cb: DcMessageCallback) -> i32 {
    let Some(h) = dc_ptr_to_handle(handle) else { return -1 };
    h.callbacks.on_message = cb; 0
}

// ============================================================================
// VideoFrame
// ============================================================================

use gkit_media::video::buffer::{
    I420Buffer, I422Buffer, I444Buffer, NV12Buffer, VideoBuffer, VideoBufferType, VideoFormatType,
};
use gkit_media::video::convert::{argb_to_i420, i420_to_argb, i420_to_nv12, i420_to_nv21, nv21_to_i420, to_i420 as convert_to_i420};
use gkit_media::video::frame::{BoxVideoFrame, VideoFrame as Vf, VideoRotation};
use gkit_media::video::transform::{i420_crop, i420_rotate, i420_scale};

struct VideoFrameHandle {
    frame: BoxVideoFrame,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_create(
    width: u32, height: u32,
) -> *mut std::ffi::c_void {
    let buf = I420Buffer::new(width, height);
    let frame = BoxVideoFrame::new(Box::new(buf));
    Box::into_raw(Box::new(VideoFrameHandle { frame })) as *mut std::ffi::c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_create_nv12(
    width: u32, height: u32,
) -> *mut std::ffi::c_void {
    let buf = NV12Buffer::new(width, height);
    let frame = BoxVideoFrame::new(Box::new(buf));
    Box::into_raw(Box::new(VideoFrameHandle { frame })) as *mut std::ffi::c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_destroy(handle: *mut std::ffi::c_void) { unsafe {
    if handle.is_null() { return; }
    let _ = Box::from_raw(handle as *mut VideoFrameHandle);
}}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_get_width(handle: *mut std::ffi::c_void) -> u32 {
    if handle.is_null() { return 0; }
    unsafe { (*(handle as *mut VideoFrameHandle)).frame.width() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_get_height(handle: *mut std::ffi::c_void) -> u32 {
    if handle.is_null() { return 0; }
    unsafe { (*(handle as *mut VideoFrameHandle)).frame.height() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_get_rotation(handle: *mut std::ffi::c_void) -> i32 {
    if handle.is_null() { return 0; }
    unsafe { (*(handle as *mut VideoFrameHandle)).frame.rotation as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_set_rotation(
    handle: *mut std::ffi::c_void, rotation: i32,
) { unsafe {
    if handle.is_null() { return; }
    let r = match rotation {
        90 => VideoRotation::Rotation90,
        180 => VideoRotation::Rotation180,
        270 => VideoRotation::Rotation270,
        _ => VideoRotation::Rotation0,
    };
    (*(handle as *mut VideoFrameHandle)).frame.rotation = r;
}}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_get_timestamp(
    handle: *mut std::ffi::c_void,
) -> i64 {
    if handle.is_null() { return 0; }
    unsafe { (*(handle as *mut VideoFrameHandle)).frame.timestamp_us }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_set_timestamp(
    handle: *mut std::ffi::c_void, ts: i64,
) { unsafe {
    if handle.is_null() { return; }
    (*(handle as *mut VideoFrameHandle)).frame.timestamp_us = ts;
}}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_get_buffer_type(
    handle: *mut std::ffi::c_void,
) -> i32 {
    if handle.is_null() { return -1; }
    unsafe {
        match (*(handle as *mut VideoFrameHandle)).frame.buffer.buffer_type() {
            VideoBufferType::I420 => 0,
            VideoBufferType::I420A => 1,
            VideoBufferType::I422 => 2,
            VideoBufferType::I444 => 3,
            VideoBufferType::I010 => 4,
            VideoBufferType::NV12 => 5,
            VideoBufferType::Native => 6,
        }
    }
}

/// Get I420 plane data pointers. Caller provides buffers; data is copied.
/// Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_get_i420_planes(
    handle: *mut std::ffi::c_void,
    out_data_y: *mut u8, out_stride_y: *mut u32,
    out_data_u: *mut u8, out_stride_u: *mut u32,
    out_data_v: *mut u8, out_stride_v: *mut u32,
) -> i32 {
    if handle.is_null() { return -1; }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        // Convert to I420 if needed
        let i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return -1,
        };
        if !out_data_y.is_null() {
            let len = (i420.stride_y * i420.height) as usize;
            std::ptr::copy_nonoverlapping(i420.data_y.as_ptr(), out_data_y, len.min(1024*1024));
        }
        if !out_stride_y.is_null() { *out_stride_y = i420.stride_y; }
        if !out_data_u.is_null() {
            let len = (i420.stride_u * i420.chroma_height()) as usize;
            std::ptr::copy_nonoverlapping(i420.data_u.as_ptr(), out_data_u, len.min(1024*1024));
        }
        if !out_stride_u.is_null() { *out_stride_u = i420.stride_u; }
        if !out_data_v.is_null() {
            let len = (i420.stride_v * i420.chroma_height()) as usize;
            std::ptr::copy_nonoverlapping(i420.data_v.as_ptr(), out_data_v, len.min(1024*1024));
        }
        if !out_stride_v.is_null() { *out_stride_v = i420.stride_v; }
    }
    0
}

/// Scale video frame. Returns new handle or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_scale(
    handle: *mut std::ffi::c_void, scaled_width: u32, scaled_height: u32,
) -> *mut std::ffi::c_void {
    if handle.is_null() { return std::ptr::null_mut(); }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        let i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return std::ptr::null_mut(),
        };
        let scaled = match i420_scale(&i420, scaled_width, scaled_height) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let new_frame = BoxVideoFrame::new(Box::new(scaled));
        Box::into_raw(Box::new(VideoFrameHandle { frame: new_frame })) as *mut std::ffi::c_void
    }
}

/// Crop video frame. Returns new handle or null.
/// x, y, w, h must be even-aligned for I420.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_crop(
    handle: *mut std::ffi::c_void, x: u32, y: u32, w: u32, h: u32,
) -> *mut std::ffi::c_void {
    if handle.is_null() { return std::ptr::null_mut(); }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        let i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return std::ptr::null_mut(),
        };
        let cropped = match i420_crop(&i420, x, y, w, h) {
            Ok(c) => c,
            Err(_) => return std::ptr::null_mut(),
        };
        let new_frame = BoxVideoFrame::new(Box::new(cropped));
        Box::into_raw(Box::new(VideoFrameHandle { frame: new_frame })) as *mut std::ffi::c_void
    }
}

/// Rotate video frame. Returns new handle or null.
/// rotation: 0, 90, 180, 270.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_rotate(
    handle: *mut std::ffi::c_void, rotation: u32,
) -> *mut std::ffi::c_void {
    if handle.is_null() { return std::ptr::null_mut(); }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        let i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return std::ptr::null_mut(),
        };
        let rotated = match i420_rotate(&i420, rotation) {
            Ok(r) => r,
            Err(_) => return std::ptr::null_mut(),
        };
        let new_frame = BoxVideoFrame::new(Box::new(rotated));
        Box::into_raw(Box::new(VideoFrameHandle { frame: new_frame })) as *mut std::ffi::c_void
    }
}

/// Create I420 video frame from RGBA pixel data.
/// Returns new handle or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_argb_to_i420(
    data: *const u8, width: u32, height: u32, stride: u32,
) -> *mut std::ffi::c_void {
    if data.is_null() { return std::ptr::null_mut(); }
    unsafe {
        let len = (stride * height) as usize;
        let rgba = std::slice::from_raw_parts(data, len);
        let i420 = match argb_to_i420(rgba, width, height, stride) {
            Ok(buf) => buf,
            Err(_) => return std::ptr::null_mut(),
        };
        let frame = BoxVideoFrame::new(Box::new(i420));
        Box::into_raw(Box::new(VideoFrameHandle { frame })) as *mut std::ffi::c_void
    }
}

/// Convert I420 video frame to RGBA pixel data.
/// format: 0=ARGB, 1=BGRA, 2=ABGR, 3=RGBA.
/// Returns 0 on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_i420_to_argb(
    handle: *mut std::ffi::c_void, out_data: *mut u8, out_stride: u32, format: i32,
) -> i32 {
    if handle.is_null() || out_data.is_null() { return -1; }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        let i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return -1,
        };
        let fmt = match format {
            0 => VideoFormatType::ARGB,
            1 => VideoFormatType::BGRA,
            2 => VideoFormatType::ABGR,
            _ => VideoFormatType::RGBA,
        };
        let len = (out_stride * i420.height) as usize;
        let out = std::slice::from_raw_parts_mut(out_data, len);
        i420_to_argb(&i420, out, out_stride, fmt);
    }
    0
}

/// Convert I420 frame to NV12. Returns new NV12 handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_i420_to_nv12(
    handle: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    if handle.is_null() { return std::ptr::null_mut(); }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        let i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return std::ptr::null_mut(),
        };
        let nv12 = i420_to_nv12(&i420);
        let new_frame = BoxVideoFrame::new(Box::new(nv12));
        Box::into_raw(Box::new(VideoFrameHandle { frame: new_frame })) as *mut std::ffi::c_void
    }
}

/// Convert NV12 frame back to I420. Returns new I420 handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_nv12_to_i420(
    handle: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    if handle.is_null() { return std::ptr::null_mut(); }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        let i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return std::ptr::null_mut(),
        };
        let new_frame = BoxVideoFrame::new(Box::new(i420));
        Box::into_raw(Box::new(VideoFrameHandle { frame: new_frame })) as *mut std::ffi::c_void
    }
}

/// Convert I420 frame to NV21. Returns new NV21 handle or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_i420_to_nv21(
    handle: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    if handle.is_null() { return std::ptr::null_mut(); }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        let i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return std::ptr::null_mut(),
        };
        let nv21 = i420_to_nv21(&i420);
        let new_frame = BoxVideoFrame::new(Box::new(nv21));
        Box::into_raw(Box::new(VideoFrameHandle { frame: new_frame })) as *mut std::ffi::c_void
    }
}

/// Convert NV21 frame back to I420. Returns new I420 handle or null.
/// NV21 stores chroma as V-U (swapped vs NV12's U-V), so we correct the U/V planes after to_i420.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_frame_nv21_to_i420(
    handle: *mut std::ffi::c_void,
) -> *mut std::ffi::c_void {
    if handle.is_null() { return std::ptr::null_mut(); }
    unsafe {
        let frame = &(*(handle as *mut VideoFrameHandle)).frame;
        let mut i420 = match frame.buffer.to_i420() {
            Ok(b) => b,
            Err(_) => return std::ptr::null_mut(),
        };
        // NV21 data → to_i420 reads V-U as U-V → swap U/V planes back
        std::mem::swap(&mut i420.data_u, &mut i420.data_v);
        std::mem::swap(&mut i420.stride_u, &mut i420.stride_v);
        let new_frame = BoxVideoFrame::new(Box::new(i420));
        Box::into_raw(Box::new(VideoFrameHandle { frame: new_frame })) as *mut std::ffi::c_void
    }
}

// ============================================================================
// VideoFrameGenerator C FFI
// ============================================================================

use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};
use std::sync::Mutex;

struct GeneratorHandle {
    generator: Mutex<VideoFrameGenerator>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_source_create_generator(
    width: u32, height: u32, fps: u32,
) -> *mut std::ffi::c_void {
    let generator = VideoFrameGenerator::new(width, height, fps);
    Box::into_raw(Box::new(GeneratorHandle { generator: Mutex::new(generator) })) as *mut std::ffi::c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_source_destroy(handle: *mut std::ffi::c_void) { unsafe {
    if handle.is_null() { return; }
    let _ = Box::from_raw(handle as *mut GeneratorHandle);
}}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_source_start(handle: *mut std::ffi::c_void) -> i32 {
    if handle.is_null() { return -1; }
    unsafe {
        let h = &*(handle as *mut GeneratorHandle);
        h.generator.lock().unwrap().start();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_source_stop(handle: *mut std::ffi::c_void) {
    if handle.is_null() { return; }
    unsafe {
        let h = &*(handle as *mut GeneratorHandle);
        h.generator.lock().unwrap().stop();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_source_is_running(handle: *mut std::ffi::c_void) -> i32 {
    if handle.is_null() { return 0; }
    unsafe {
        let h = &*(handle as *mut GeneratorHandle);
        h.generator.lock().unwrap().is_running() as i32
    }
}

pub type VideoFrameCallback = unsafe extern "C" fn(frame_handle: *mut std::ffi::c_void, user_data: *mut std::ffi::c_void);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_video_source_set_frame_callback(
    handle: *mut std::ffi::c_void,
    cb: VideoFrameCallback,
    user_data: *mut std::ffi::c_void,
) -> i32 {
    if handle.is_null() { return -1; }
    unsafe {
        let h = &*(handle as *mut GeneratorHandle);
        let mut g = h.generator.lock().unwrap();
        let user = user_data as usize;
        struct CbSink {
            cb: VideoFrameCallback,
            user_data: usize,
        }
        unsafe impl Send for CbSink {}
        impl gkit_media::video::source_sink::VideoSink<gkit_media::video::frame::BoxVideoFrame> for CbSink {
            fn on_frame(&self, _frame: &gkit_media::video::frame::BoxVideoFrame) {
                let i420 = _frame.buffer.to_i420().ok();
                let handle = match i420 {
                    Some(buf) => Box::into_raw(Box::new(VideoFrameHandle {
                        frame: gkit_media::video::frame::BoxVideoFrame::new(Box::new(buf)),
                    })) as *mut std::ffi::c_void,
                    None => std::ptr::null_mut(),
                };
                unsafe { (self.cb)(handle, self.user_data as *mut std::ffi::c_void); }
            }
        }
        g.add_or_update_sink(
            Box::new(CbSink { cb, user_data: user }),
            gkit_media::video::source_sink::VideoSinkWants { is_active: true, ..Default::default() },
        );
        0
    }
}
