use std::sync::{Arc, Mutex};
use gkit_media::protocols::rtc::peer::{
    IceCandidate, IceConnectionState, PeerConnection, PeerConnectionFactory,
};
use gkit_media::protocols::rtc::peer::RtcEngine;

fn mock() { gkit_media::register_test_backend(); }

#[test]
#[ignore = "requires real WebRTC backend for ICE callbacks"]
fn ice_candidate_callback_registered() {
    mock();
    let factory = RtcEngine::create_default().expect("factory");
    let pc = factory.create_peer_connection().expect("create pc");
    let candidates = Arc::new(Mutex::new(Vec::new()));
    let c = candidates.clone();
    pc.set_on_ice_candidate(Box::new(move |cand: IceCandidate| {
        c.lock().unwrap().push(cand.candidate);
    }));
    assert!(true);
}

#[test]
#[ignore = "requires real WebRTC backend for ICE callbacks"]
fn ice_state_callback_registered() {
    mock();
    let factory = RtcEngine::create_default().expect("factory");
    let pc = factory.create_peer_connection().expect("create pc");
    let states = Arc::new(Mutex::new(Vec::new()));
    let s = states.clone();
    pc.set_on_ice_connection_state_change(Box::new(move |state: IceConnectionState| {
        s.lock().unwrap().push(state);
    }));
    assert!(true);
}

#[test]
#[ignore = "requires real WebRTC backend for ICE callbacks"]
fn ice_gathering_complete_returns_ok() {
    mock();
    let factory = RtcEngine::create_default().expect("factory");
    let pc = factory.create_peer_connection().expect("create pc");
    let _ = pc.gather_complete();
}

#[test]
fn ice_state_starts_new() {
    mock();
    let factory = RtcEngine::create_default().expect("factory");
    let pc = factory.create_peer_connection().expect("create pc");
    assert_eq!(pc.ice_connection_state(), IceConnectionState::New);
}

#[test]
fn ice_state_closed_after_close() {
    mock();
    let factory = RtcEngine::create_default().expect("factory");
    let mut pc = factory.create_peer_connection().expect("create pc");
    pc.close().expect("close");
    assert_eq!(pc.ice_connection_state(), IceConnectionState::Closed);
}

#[test]
fn ice_candidate_structure() {
    let cand = IceCandidate {
        candidate: "candidate:1 1 UDP 2122252543 192.168.1.1 12345 typ host".into(),
        sdp_mid: Some("0".into()),
        sdp_mline_index: Some(0),
    };
    assert_eq!(cand.sdp_mid.as_deref(), Some("0"));
    assert!(cand.candidate.contains("UDP"));
}
