#[cfg(all(feature = "backend-native-webrtc-rs", feature = "backend-native-google"))]
compile_error!("Cannot enable both webrtc-rs and Google libwebrtc backends at the same time");

#[cfg(feature = "backend-native-google")]
mod google;
#[cfg(feature = "backend-native-google")]
pub use google::*;

#[cfg(any(
    all(feature = "backend-native", not(feature = "backend-native-webrtc-rs"), not(feature = "backend-native-google")),
    feature = "backend-native-webrtc-rs"
))]
mod webrtc_rs;
#[cfg(any(
    all(feature = "backend-native", not(feature = "backend-native-webrtc-rs"), not(feature = "backend-native-google")),
    feature = "backend-native-webrtc-rs"
))]
pub use webrtc_rs::*;
