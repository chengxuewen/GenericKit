use gkit_core::core_hello;

pub fn media_hello() {
    core_hello();
    println!("media_hello!");
}

pub mod capture;
pub mod protocols;
pub mod video;

// build-sys: LiveKit webrtc-sys FFI (requires libwebrtc C++ binary)
// Not yet integrated — google_lk import paths need porting
// #[cfg(feature = "backend-native-google")]
// #[path = "build-sys/mod.rs"]
// pub mod build_sys;

pub fn make_peer_connection() -> Box<dyn protocols::rtc::client::core::PeerConnection> {
    use protocols::rtc::client::engine::RtcEngine;
    RtcEngine::create_default()
        .expect("no RTC backend registered")
        .create_peer_connection()
        .expect("failed to create PeerConnection")
}

pub fn make_peer_connection_with_backend(name: &str) -> protocols::rtc::client::core::MediaResult<Box<dyn protocols::rtc::client::core::PeerConnection>> {
    use protocols::rtc::client::engine::RtcEngine;
    RtcEngine::create(name)?.create_peer_connection()
}
