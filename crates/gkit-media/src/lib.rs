use gkit_core::core_hello;

pub fn media_hello() {
    core_hello();
    println!("media_hello!");
}

pub mod webrtc;
pub mod video;

// build-sys: LiveKit webrtc-sys FFI (requires libwebrtc C++ binary)
// #[cfg(feature = "backend-native-google")]
// #[path = "build-sys/mod.rs"]
// pub mod build_sys;

// --- backend-agnostic factory functions (used by C FFI) ---

use webrtc::client::core::{DataChannel, PeerConnection};

#[cfg(all(feature = "backend-native", not(feature = "backend-native-google")))]
pub fn make_peer_connection() -> Box<dyn PeerConnection> {
    Box::new(webrtc::client::native::NativePeerConnection::new())
}

#[cfg(feature = "backend-native-google")]
pub fn make_peer_connection() -> Box<dyn PeerConnection> {
    Box::new(webrtc::client::native::GooglePeerConnection::new())
}

#[cfg(all(feature = "backend-native", not(feature = "backend-native-google")))]
pub fn make_data_channel(label: &str) -> Box<dyn DataChannel> {
    Box::new(webrtc::client::native::NativeDataChannel::new(label))
}

#[cfg(feature = "backend-native-google")]
pub fn make_data_channel(label: &str) -> Box<dyn DataChannel> {
    Box::new(webrtc::client::native::GoogleDataChannel::new(label))
}
