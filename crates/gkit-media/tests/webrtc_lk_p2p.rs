use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gkit_media::protocols::rtc::client::core::{
    IceCandidate, IceConnectionState, IceServer, RtcConfiguration,
};
use gkit_media::protocols::rtc::client::engine::RtcEngine;

const ICE_TIMEOUT_SECS: u64 = 30;

#[test]
fn livekit_p2p_offer_answer_ice() {
    let config = RtcConfiguration {
        ice_servers: vec![
            IceServer {
                urls: vec!["stun:stun.l.google.com:19302".into()],
                username: None,
                credential: None,
            },
        ],
        ..Default::default()
    };

    let factory = RtcEngine::create("google").expect("google backend not registered");
    let mut alice = factory
        .create_peer_connection_with_config(&config)
        .expect("alice");
    let mut bob = factory
        .create_peer_connection_with_config(&config)
        .expect("bob");

    let alice_state = Arc::new(Mutex::new(IceConnectionState::New));
    let bob_state = Arc::new(Mutex::new(IceConnectionState::New));
    let alice_candidates = Arc::new(Mutex::new(Vec::new()));
    let bob_candidates = Arc::new(Mutex::new(Vec::new()));

    {
        let as_ = alice_state.clone();
        alice.set_on_ice_connection_state_change(Box::new(move |s| {
            eprintln!("[alice] ICE state: {:?}", s);
            *as_.lock().unwrap() = s;
        }));
    }
    {
        let bs = bob_state.clone();
        bob.set_on_ice_connection_state_change(Box::new(move |s| {
            eprintln!("[bob] ICE state: {:?}", s);
            *bs.lock().unwrap() = s;
        }));
    }
    {
        let ac = alice_candidates.clone();
        alice.set_on_ice_candidate(Box::new(move |c| {
            eprintln!("[alice] candidate: mid={:?} line={:?}", c.sdp_mid, c.sdp_mline_index);
            ac.lock().unwrap().push(c);
        }));
    }
    {
        let bc = bob_candidates.clone();
        bob.set_on_ice_candidate(Box::new(move |c| {
            eprintln!("[bob] candidate: mid={:?} line={:?}", c.sdp_mid, c.sdp_mline_index);
            bc.lock().unwrap().push(c);
        }));
    }

    // Add DataChannels — m= line required to trigger ICE candidate gathering
    let _alice_dc = alice.create_data_channel("test").expect("alice dc");
    let _bob_dc = bob.create_data_channel("test").expect("bob dc");

    // Offer / Answer exchange
    let offer = alice.create_offer().expect("alice offer");
    alice.set_local_description(&offer).expect("alice setLocal");
    bob.set_remote_description(&offer).expect("bob setRemote");

    let answer = bob.create_answer().expect("bob answer");
    bob.set_local_description(&answer).expect("bob setLocal");
    alice.set_remote_description(&answer).expect("alice setRemote");

    // Wait for ICE gathering to produce candidates
    std::thread::sleep(Duration::from_secs(5));

    // Exchange ICE candidates — loop until both sides have gathered
    let start = Instant::now();
    loop {
        let alice_ics: Vec<IceCandidate> = alice_candidates.lock().unwrap().drain(..).collect();
        let bob_ics: Vec<IceCandidate> = bob_candidates.lock().unwrap().drain(..).collect();

        eprintln!(
            "[exchange] alice={} bob={}",
            alice_ics.len(),
            bob_ics.len()
        );

        for c in &alice_ics {
            bob.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or(""))
                .ok();
        }
        for c in &bob_ics {
            alice
                .add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or(""))
                .ok();
        }

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
                "ICE timeout after {}s: alice={:?}({} candidates) bob={:?}({} candidates)",
                ICE_TIMEOUT_SECS,
                as_,
                alice_candidates.lock().unwrap().len(),
                bs,
                bob_candidates.lock().unwrap().len()
            );
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    assert_eq!(alice.ice_connection_state(), IceConnectionState::Connected);
    assert_eq!(bob.ice_connection_state(), IceConnectionState::Connected);

    alice.close().ok();
    bob.close().ok();
    assert_eq!(alice.ice_connection_state(), IceConnectionState::Closed);
}
