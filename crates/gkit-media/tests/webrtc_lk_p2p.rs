use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gkit_media::protocols::rtc::client::core::{IceCandidate, IceConnectionState, PeerConnection};
use gkit_media::protocols::rtc::client::engine::RtcEngine;

const ICE_TIMEOUT_SECS: u64 = 20;

#[test]
fn livekit_p2p_offer_answer_ice() {
    let factory = RtcEngine::create("google").expect("google backend not registered");
    let mut alice = factory.create_peer_connection().expect("alice");
    let mut bob = factory.create_peer_connection().expect("bob");

    let alice_state = Arc::new(Mutex::new(IceConnectionState::New));
    let bob_state = Arc::new(Mutex::new(IceConnectionState::New));
    let alice_candidates = Arc::new(Mutex::new(Vec::new()));
    let bob_candidates = Arc::new(Mutex::new(Vec::new()));

    {
        let as_ = alice_state.clone();
        alice.set_on_ice_connection_state_change(Box::new(move |s| {
            *as_.lock().unwrap() = s;
        }));
    }
    {
        let bs = bob_state.clone();
        bob.set_on_ice_connection_state_change(Box::new(move |s| {
            *bs.lock().unwrap() = s;
        }));
    }
    {
        let ac = alice_candidates.clone();
        alice.set_on_ice_candidate(Box::new(move |c| {
            ac.lock().unwrap().push(c);
        }));
    }
    {
        let bc = bob_candidates.clone();
        bob.set_on_ice_candidate(Box::new(move |c| {
            bc.lock().unwrap().push(c);
        }));
    }

    // Offer / Answer exchange
    let offer = alice.create_offer().expect("alice offer");
    alice.set_local_description(&offer).expect("alice setLocal");
    bob.set_remote_description(&offer).expect("bob setRemote");

    let answer = bob.create_answer().expect("bob answer");
    bob.set_local_description(&answer).expect("bob setLocal");
    alice.set_remote_description(&answer).expect("alice setRemote");

    // Exchange ICE candidates
    let alice_ics: Vec<IceCandidate> = alice_candidates.lock().unwrap().drain(..).collect();
    let bob_ics: Vec<IceCandidate> = bob_candidates.lock().unwrap().drain(..).collect();

    for c in &alice_ics {
        bob.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or(""))
            .ok();
    }
    for c in &bob_ics {
        alice
            .add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or(""))
            .ok();
    }

    // Also exchange any late-arriving candidates
    std::thread::sleep(Duration::from_millis(200));
    let alice_late: Vec<IceCandidate> = alice_candidates.lock().unwrap().drain(..).collect();
    let bob_late: Vec<IceCandidate> = bob_candidates.lock().unwrap().drain(..).collect();
    for c in &alice_late {
        bob.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or(""))
            .ok();
    }
    for c in &bob_late {
        alice
            .add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or(""))
            .ok();
    }

    // Wait for ICE connection
    let start = Instant::now();
    loop {
        let as_ = *alice_state.lock().unwrap();
        let bs = *bob_state.lock().unwrap();
        if (as_ == IceConnectionState::Connected || as_ == IceConnectionState::Completed)
            && (bs == IceConnectionState::Connected || bs == IceConnectionState::Completed)
        {
            break;
        }
        if start.elapsed() > Duration::from_secs(ICE_TIMEOUT_SECS) {
            alice.close().ok();
            bob.close().ok();
            panic!(
                "ICE connection timeout after {}s: alice={:?} bob={:?}",
                ICE_TIMEOUT_SECS, as_, bs
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    assert_eq!(alice.ice_connection_state(), IceConnectionState::Connected);
    assert_eq!(bob.ice_connection_state(), IceConnectionState::Connected);

    alice.close().ok();
    bob.close().ok();
    assert_eq!(alice.ice_connection_state(), IceConnectionState::Closed);
}
