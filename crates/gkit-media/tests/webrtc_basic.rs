// W3C PeerConnection basic lifecycle tests.
// Backend-agnostic: uses factory functions, works with all backends.

use gkit_media::protocols::rtc::peer::PeerConnection;

#[test]
fn create_and_close() {
    let mut pc = gkit_media::make_peer_connection();
    assert!(pc.close().is_ok());
}

#[test]
fn set_local_description() {
    let mut pc = gkit_media::make_peer_connection();
    let offer = pc.create_offer().expect("create_offer failed");
    assert_eq!(offer.sdp_type, "offer");
    pc.set_local_description(&offer)
        .expect("set_local_description failed");
}

#[test]
fn set_remote_description() {
    let mut pc = gkit_media::make_peer_connection();
    let answer = pc.create_answer().expect("create_answer failed");
    assert_eq!(answer.sdp_type, "answer");
    pc.set_remote_description(&answer)
        .expect("set_remote_description failed");
}

#[test]
fn error_on_closed_connection() {
    let mut pc = gkit_media::make_peer_connection();
    pc.close().expect("close failed");
    assert!(pc.create_offer().is_err());
    assert!(pc.create_answer().is_err());
}

#[test]
fn ice_candidate_handling() {
    let mut pc = gkit_media::make_peer_connection();
    pc.add_ice_candidate("candidate:0 1 UDP 2122252543 192.168.1.1 12345 typ host", "0")
        .expect("add_ice_candidate failed");
}
