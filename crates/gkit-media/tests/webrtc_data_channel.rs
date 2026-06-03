// W3C DataChannel tests.
// Backend-agnostic: uses factory functions.

use gkit_media::protocols::rtc::peer::PeerConnection;

#[test]
fn create_data_channel() {
    let pc = gkit_media::make_peer_connection();
    let dc = pc.create_data_channel("test").expect("create_data_channel");
    assert_eq!(dc.label(), "test");
}

#[test]
#[ignore = "requires P2P connection (offer/answer exchange) before data channel opens"]
fn send_text() {
    let pc = gkit_media::make_peer_connection();
    let dc = pc.create_data_channel("chat").expect("create_data_channel");
    dc.send_text("hello").expect("send_text");
}

#[test]
#[ignore = "requires P2P connection (offer/answer exchange) before data channel opens"]
fn send_bytes() {
    let pc = gkit_media::make_peer_connection();
    let dc = pc.create_data_channel("binary").expect("create_data_channel");
    dc.send_bytes(&[0u8, 1, 2, 3]).expect("send_bytes");
}

#[test]
fn close_data_channel() {
    let pc = gkit_media::make_peer_connection();
    let mut dc = pc.create_data_channel("close-me").expect("create_data_channel");
    dc.close().expect("close");
    assert!(dc.send_text("after close").is_err());
}

#[test]
fn multiple_data_channels() {
    let pc = gkit_media::make_peer_connection();
    let dc1 = pc.create_data_channel("ch1").expect("create ch1");
    let dc2 = pc.create_data_channel("ch2").expect("create ch2");
    assert_eq!(dc1.label(), "ch1");
    assert_eq!(dc2.label(), "ch2");
}

#[test]
fn channel_error_on_closed_peer() {
    let mut pc = gkit_media::make_peer_connection();
    pc.close().expect("close pc");
    assert!(pc.create_data_channel("after close").is_err());
}
