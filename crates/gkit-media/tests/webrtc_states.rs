// W3C state machine transition tests.
// Backend-agnostic: uses factory functions and core types.

use gkit_media::protocols::rtc::peer::{DataChannel, IceConnectionState, PeerConnection, PeerConnectionFactory};

#[test]
fn ice_state_initial() {
    let pc = gkit_media::make_peer_connection();
    assert_eq!(pc.ice_connection_state(), IceConnectionState::New);
}

#[test]
fn ice_state_after_close() {
    let mut pc = gkit_media::make_peer_connection();
    pc.close().expect("close");
    assert_eq!(pc.ice_connection_state(), IceConnectionState::Closed);
}

#[test]
#[ignore = "requires P2P connection (offer/answer) for data channel to reach Open state"]
fn data_channel_ready_state_after_close() {
    use gkit_media::protocols::rtc::peer::DataChannelState;
    let pc = gkit_media::make_peer_connection();
    let mut dc = pc.create_data_channel("dc").expect("create_data_channel");
    assert_eq!(dc.ready_state(), DataChannelState::Open);
    dc.close().expect("close dc");
    assert_eq!(dc.ready_state(), DataChannelState::Closed);
}

#[test]
fn factory_creates_independent_connections() {
    let pc1 = gkit_media::make_peer_connection();
    let pc2 = gkit_media::make_peer_connection();
    assert_eq!(pc2.ice_connection_state(), IceConnectionState::New);
    drop(pc1);
    let _o = pc2.create_offer().expect("pc2 create_offer");
}
