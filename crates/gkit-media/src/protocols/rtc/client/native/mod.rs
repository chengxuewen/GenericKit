#[cfg(feature = "backend-native-webrtc-rs")]
mod webrtc_rs;
#[cfg(feature = "backend-native-webrtc-rs")]
pub use webrtc_rs::*;

#[cfg(feature = "backend-native-google")]
mod google;
#[cfg(feature = "backend-native-google")]
pub use google::*;

// google_lk port status: 28 remaining errors (macro metavar, trait bounds)
// #[cfg(feature = "backend-native-google")]
// mod google_lk;

#[cfg(not(any(
    feature = "backend-native-webrtc-rs",
    feature = "backend-native-google"
)))]
compile_error!("at least one native RTC backend feature required when backend-native is enabled");
