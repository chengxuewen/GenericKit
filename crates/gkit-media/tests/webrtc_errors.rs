// Error handling tests.
// Backend-agnostic: uses factory functions.

use gkit_media::protocols::rtc::client::core::PeerConnection;

#[test]
fn error_message_is_descriptive() {
    let mut pc = gkit_media::make_peer_connection();
    pc.close().unwrap();
    let err = pc.create_offer().unwrap_err();
    assert!(err.to_string().contains("closed"));
}

#[test]
fn double_close_is_idempotent() {
    let mut pc = gkit_media::make_peer_connection();
    assert!(pc.close().is_ok());
    assert!(pc.close().is_ok());
}

#[test]
fn operations_on_closed_peer_all_fail() {
    let mut pc = gkit_media::make_peer_connection();
    pc.close().unwrap();
    assert!(pc.create_offer().is_err());
    assert!(pc.create_answer().is_err());
    let desc = gkit_media::protocols::rtc::client::core::SessionDescription {
        sdp_type: "offer".into(),
        sdp: String::new(),
    };
    assert!(pc.set_local_description(&desc).is_err());
    assert!(pc.set_remote_description(&desc).is_err());
    assert!(pc.add_ice_candidate("", "").is_err());
}
