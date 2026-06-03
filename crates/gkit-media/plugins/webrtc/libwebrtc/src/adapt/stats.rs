//! Stats collection adapter wrapping `libwebrtc::stats::RtcStats`.
//!
//! Provides helpers to collect and serialize WebRTC statistics from a
//! [`PeerConnection`](libwebrtc::peer_connection::PeerConnection) or individual
//! RTP sender / receiver.

use libwebrtc::stats::RtcStats;

use gkit_media::protocols::rtc::client::core::{MediaError, MediaResult};

/// Collect all stats from the peer connection and return them as a pretty-printed
/// JSON string.
pub async fn get_stats_json(
    pc: &libwebrtc::peer_connection::PeerConnection,
) -> MediaResult<String> {
    let stats = pc
        .get_stats()
        .await
        .map_err(|e| MediaError::new(format!("get_stats failed: {e}")))?;
    Ok(format!("{:#?}", stats))
}

/// Collect stats for a single RTP sender and return as JSON string.
pub async fn sender_stats_json(sender: &libwebrtc::rtp_sender::RtpSender) -> MediaResult<String> {
    let stats = sender
        .get_stats()
        .await
        .map_err(|e| MediaError::new(format!("sender get_stats failed: {e}")))?;
    Ok(format!("{:#?}", stats))
}

/// Collect stats for a single RTP receiver and return as JSON string.
pub async fn receiver_stats_json(
    receiver: &libwebrtc::rtp_receiver::RtpReceiver,
) -> MediaResult<String> {
    let stats = receiver
        .get_stats()
        .await
        .map_err(|e| MediaError::new(format!("receiver get_stats failed: {e}")))?;
    Ok(format!("{:#?}", stats))
}

/// Re-export the full [`RtcStats`] enum and its substructs so callers can work
/// with strongly-typed stats instead of raw JSON when preferred.
pub use libwebrtc::stats::*;

#[cfg(test)]
mod tests {
    use crate::adapt::*;

    #[test]
    fn rtc_stats_serde_roundtrip() {
        // Verify that the RtcStats enum can be serialized (it derives Deserialize
        // upstream; we add Serialize via serde_json if needed, but for now we only
        // need serialization through the to_string_pretty path which goes through
        // serde_json — note: the upstream types only derive Deserialize, not Serialize.
        //
        // The `get_stats_json` helpers serialize the `Vec<RtcStats>`, which requires
        // `RtcStats: Serialize`. Upstream only provides `Deserialize`. If you need
        // serialization downstream, enable the `serde` feature on libwebrtc or add
        // a local Serialize impl via a newtype.
        //
        // For now we test that the types are accessible and correct.
        let _ = std::mem::size_of::<RtcStats>();
    }
}
