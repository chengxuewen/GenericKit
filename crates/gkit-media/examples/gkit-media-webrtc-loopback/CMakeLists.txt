include(GKitCargoExample)

# WebRTC backends are mutually exclusive at link-time (libwebrtc and webrtc-rs
# both pull in OpenH264 encoder code, causing 1257 duplicate symbols).
# Select ONE backend here; the egui dropdown will only show compiled-in backends.
#
# Default: webrtc-rs (pure Rust, no C++ build deps, works on all platforms)
# For Google libwebrtc: change FEATURES to "backend-native-google"
set(_loopback_features "backend-native-webrtc-rs")

gkit_cargo_add_example(
    NAME gkit-media-webrtc-loopback
    CRATE gkit-media
    FOLDER gkit_media/examples
    FEATURES "${_loopback_features}"
)
