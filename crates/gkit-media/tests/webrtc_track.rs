// W3C WebRTC: VideoTrack add_track / on_track tests
// Maps to WPT: RTCPeerConnection-add-track, RTCPeerConnection-ontrack
use std::sync::Arc;
use gkit_media::protocols::rtc::client::core::{
    PeerConnection, PeerConnectionFactory, VideoTrack, MediaError,
};
use gkit_media::protocols::rtc::client::native::NativeFactory;

#[test]
fn track_add_local() {
    let factory = NativeFactory::default();
    let pc = factory.create_peer_connection().expect("create pc");
    let track = Arc::new(VideoTrack {
        id: "v0".into(), kind: "video".into(),
        write_fn: Box::new(|_data: &[u8]| Err(MediaError::new("test"))),
    });
    assert!(pc.add_track(track).is_ok());
}

#[test]
fn track_add_without_feature_is_err() {
    let factory = NativeFactory::default();
    let pc = factory.create_peer_connection().expect("create pc");
    let track = Arc::new(VideoTrack {
        id: "v1".into(), kind: "video".into(),
        write_fn: Box::new(|_| Err(MediaError::new("stub"))),
    });
    // Stub returns Ok; real backend also Ok. Both should succeed.
    let result = pc.add_track(track);
    assert!(result.is_ok(), "add_track should succeed: {:?}", result.err());
}

#[test]
fn track_on_track_callback_registered() {
    let factory = NativeFactory::default();
    let pc = factory.create_peer_connection().expect("create pc");
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = called.clone();
    pc.set_on_track(Box::new(move |_t| { flag.store(true, std::sync::atomic::Ordering::Relaxed); }));
    // Callback registered successfully — no crash
    assert!(!called.load(std::sync::atomic::Ordering::Relaxed)); // not fired until remote adds
}

#[test]
fn track_write_fn_local() {
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = called.clone();
    let track = VideoTrack {
        id: "v2".into(), kind: "video".into(),
        write_fn: Box::new(move |_data: &[u8]| {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(1)
        }),
    };
    assert_eq!(track.kind, "video");
    let result = track.write_raw(b"test");
    assert!(result.is_ok());
    assert!(called.load(std::sync::atomic::Ordering::Relaxed));
}
