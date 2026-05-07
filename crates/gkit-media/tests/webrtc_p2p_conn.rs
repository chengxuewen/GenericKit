// W3C WebRTC P2P Test — real RTP/SCTP connectivity, no vnet
// cargo test -p gkit-media --test webrtc_p2p_conn -- --nocapture
// Uses google_lk backend (default); switch with --features backend-native-webrtc-rs

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use gkit_media::protocols::rtc::client::core::{
    PeerConnection, PeerConnectionFactory, IceCandidate, IceConnectionState,
    DataChannel, DataChannelState,
};
use gkit_media::protocols::rtc::client::engine::RtcEngine;

const TIMEOUT_SECS: u64 = 30;

#[test]
fn p2p_ice_connectivity() {
    let factory = RtcEngine::create_default().expect("factory");

    let mut pc1 = factory.create_peer_connection().expect("pc1");
    let mut pc2 = factory.create_peer_connection().expect("pc2");

    // ICE candidates channels
    let (tx1, rx1) = std::sync::mpsc::channel::<IceCandidate>();
    let (tx2, rx2) = std::sync::mpsc::channel::<IceCandidate>();
    pc1.set_on_ice_candidate(Box::new(move |c| { let _ = tx2.send(c); }));
    pc2.set_on_ice_candidate(Box::new(move |c| { let _ = tx1.send(c); }));

    // ICE state tracking
    let pc1_ice = Arc::new(Mutex::new(IceConnectionState::New));
    let pc2_ice = Arc::new(Mutex::new(IceConnectionState::New));
    let s1 = pc1_ice.clone(); let s2 = pc2_ice.clone();
    pc1.set_on_ice_connection_state_change(Box::new(move |s| { *s1.lock().unwrap() = s; }));
    pc2.set_on_ice_connection_state_change(Box::new(move |s| { *s2.lock().unwrap() = s; }));

    // SDP exchange
    let offer = pc1.create_offer().expect("offer");
    pc1.set_local_description(&offer).expect("pc1 set local");
    pc1.gather_complete().ok();
    let desc1 = pc1.local_description().expect("pc1 local desc");
    eprintln!("[p2p] offer SDP size={}", desc1.sdp.len());

    pc2.set_remote_description(&desc1).expect("pc2 set remote");

    let answer = pc2.create_answer().expect("answer");
    pc2.set_local_description(&answer).expect("pc2 set local");
    pc2.gather_complete().ok();
    let desc2 = pc2.local_description().expect("pc2 local desc");
    eprintln!("[p2p] answer SDP size={}", desc2.sdp.len());

    pc1.set_remote_description(&desc2).expect("pc1 set remote");

    // Exchange explicit candidates
    for c in rx2.try_iter() { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
    for c in rx1.try_iter() { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }

    eprintln!("[p2p] post-exchange: pc1_ice={:?} pc2_ice={:?}", pc1.ice_connection_state(), pc2.ice_connection_state());

    // Wait for ICE connected or timeout
    let start = Instant::now();
    loop {
        let s1 = *pc1_ice.lock().unwrap();
        let s2 = *pc2_ice.lock().unwrap();
        eprintln!("[p2p] states: pc1={:?} pc2={:?}", s1, s2);
        if s1 == IceConnectionState::Connected && s2 == IceConnectionState::Connected {
            break;
        }
        if s1 == IceConnectionState::Failed || s2 == IceConnectionState::Failed {
            panic!("ICE failed: pc1={:?} pc2={:?}", s1, s2);
        }
        if start.elapsed() > Duration::from_secs(TIMEOUT_SECS) {
            panic!("ICE timeout: pc1={:?} pc2={:?}", s1, s2);
        }
        for c in rx2.try_iter() { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
        for c in rx1.try_iter() { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
        std::thread::sleep(Duration::from_millis(200));
    }

    eprintln!("[p2p] ICE connected!");
    pc1.close().ok(); pc2.close().ok();
}
