// W3C SDP offer/answer exchange tests.
// Backend-agnostic: uses factory functions.

use gkit_media::protocols::rtc::client::core::PeerConnection;

#[test]
fn offer_answer_round_trip() {
    let mut offerer = gkit_media::make_peer_connection();
    let mut answerer = gkit_media::make_peer_connection();

    let offer = offerer.create_offer().expect("create_offer failed");
    offerer.set_local_description(&offer).expect("offerer set_local");
    answerer.set_remote_description(&offer).expect("answerer set_remote");

    let answer = answerer.create_answer().expect("create_answer failed");
    answerer.set_local_description(&answer).expect("answerer set_local");
    offerer.set_remote_description(&answer).expect("offerer set_remote");
}

#[test]
fn ice_candidate_relay() {
    let mut offerer = gkit_media::make_peer_connection();
    let mut answerer = gkit_media::make_peer_connection();
    let c = "candidate:1 1 UDP 2130706431 10.0.0.1 54321 typ srflx";
    offerer.add_ice_candidate(c, "0").expect("add_ice_candidate");
    answerer.add_ice_candidate(c, "0").expect("add_ice_candidate");
}

#[test]
fn concurrent_connections() {
    let mut pc1 = gkit_media::make_peer_connection();
    let mut pc2 = gkit_media::make_peer_connection();
    let mut pc3 = gkit_media::make_peer_connection();
    let _o1 = pc1.create_offer().unwrap();
    let _o2 = pc2.create_offer().unwrap();
    let _o3 = pc3.create_offer().unwrap();
    pc1.close().unwrap();
    pc2.close().unwrap();
    pc3.close().unwrap();
}
