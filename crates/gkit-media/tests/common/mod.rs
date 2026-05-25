#![allow(dead_code)]

use std::time::{Duration, Instant};

use gkit_media::protocols::rtc::client::core::{IceConnectionState, PeerConnection};

pub async fn wait_for_ice_connected(
    pc1: &dyn PeerConnection,
    pc2: &dyn PeerConnection,
    timeout: Duration,
) -> Result<(), String> {
    let start = Instant::now();
    loop {
        if start.elapsed() > timeout {
            return Err("timeout waiting for ICE connected".into());
        }
        let s1 = pc1.ice_connection_state();
        let s2 = pc2.ice_connection_state();
        if matches!(s1, IceConnectionState::Connected | IceConnectionState::Completed)
            && matches!(s2, IceConnectionState::Connected | IceConnectionState::Completed)
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
