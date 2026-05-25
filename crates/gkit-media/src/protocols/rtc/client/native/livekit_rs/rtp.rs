//! RTP sender / receiver / transceiver adapters.
//!
//! Thin wrappers around the corresponding libwebrtc types. The core
//! [`PeerConnection`](crate::protocols::rtc::client::core::PeerConnection)
//! trait does not expose these directly; use them by downcasting a
//! [`LiveKitPeerConnection`](super::peer_connection::LiveKitPeerConnection)
//! and calling the accessors on the inner
//! [`PeerConnection`](libwebrtc::peer_connection::PeerConnection).

use libwebrtc::rtp_receiver::RtpReceiver as LkRtpReceiver;
use libwebrtc::rtp_sender::RtpSender as LkRtpSender;
use libwebrtc::rtp_transceiver::RtpTransceiver as LkRtpTransceiver;
use libwebrtc::stats::RtcStats;

use crate::protocols::rtc::client::core::{MediaError, MediaResult};

// ---------------------------------------------------------------------------
// Re-export underlying libwebrtc RTP types for convenience
// ---------------------------------------------------------------------------

/// Wraps a libwebrtc [`RtpSender`](LkRtpSender).
pub struct RtpSenderHandle {
    inner: LkRtpSender,
}

impl RtpSenderHandle {
    pub fn new(inner: LkRtpSender) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &LkRtpSender {
        &self.inner
    }

    pub fn into_inner(self) -> LkRtpSender {
        self.inner
    }

    pub fn track(
        &self,
    ) -> Option<libwebrtc::media_stream_track::MediaStreamTrack> {
        self.inner.track()
    }

    pub fn parameters(&self) -> RtpParameters {
        self.inner.parameters()
    }

    pub async fn get_stats(&self) -> MediaResult<Vec<RtcStats>> {
        self.inner
            .get_stats()
            .await
            .map_err(|e| MediaError::new(format!("rtp_sender get_stats: {e}")))
    }
}

/// Wraps a libwebrtc [`RtpReceiver`](LkRtpReceiver).
pub struct RtpReceiverHandle {
    inner: LkRtpReceiver,
}

impl RtpReceiverHandle {
    pub fn new(inner: LkRtpReceiver) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &LkRtpReceiver {
        &self.inner
    }

    pub fn into_inner(self) -> LkRtpReceiver {
        self.inner
    }

    pub fn track(
        &self,
    ) -> Option<libwebrtc::media_stream_track::MediaStreamTrack> {
        self.inner.track()
    }

    pub fn parameters(&self) -> RtpParameters {
        self.inner.parameters()
    }

    pub async fn get_stats(&self) -> MediaResult<Vec<RtcStats>> {
        self.inner
            .get_stats()
            .await
            .map_err(|e| MediaError::new(format!("rtp_receiver get_stats: {e}")))
    }
}

/// Wraps a libwebrtc [`RtpTransceiver`](LkRtpTransceiver).
pub struct RtpTransceiverHandle {
    inner: LkRtpTransceiver,
}

impl RtpTransceiverHandle {
    pub fn new(inner: LkRtpTransceiver) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &LkRtpTransceiver {
        &self.inner
    }

    pub fn mid(&self) -> Option<String> {
        self.inner.mid()
    }

    pub fn sender(&self) -> LkRtpSender {
        self.inner.sender()
    }

    pub fn receiver(&self) -> LkRtpReceiver {
        self.inner.receiver()
    }

    pub fn direction(
        &self,
    ) -> libwebrtc::rtp_transceiver::RtpTransceiverDirection {
        self.inner.direction()
    }

    pub fn current_direction(
        &self,
    ) -> Option<libwebrtc::rtp_transceiver::RtpTransceiverDirection> {
        self.inner.current_direction()
    }
}

// ---------------------------------------------------------------------------
// Re-export libwebrtc RTP parameter types
// ---------------------------------------------------------------------------

/// RTP codec parameters.
pub use libwebrtc::rtp_parameters::RtpCodecParameters;
/// RTP encoding parameters (simulcast / SVC config).
pub use libwebrtc::rtp_parameters::RtpEncodingParameters;
/// RTP header extension parameters.
pub use libwebrtc::rtp_parameters::RtpHeaderExtensionParameters;
/// Full RTP parameters (codecs + extensions + RTCP).
pub use libwebrtc::rtp_parameters::RtpParameters;
/// RTCP parameters.
pub use libwebrtc::rtp_parameters::RtcpParameters;
/// RTP codec capabilities.
pub use libwebrtc::rtp_parameters::RtpCodecCapability;
/// RTP capabilities (codecs + header extensions).
pub use libwebrtc::rtp_parameters::RtpCapabilities;
/// RTP transceiver direction.
pub use libwebrtc::rtp_transceiver::RtpTransceiverDirection;
/// RTP transceiver init options.
pub use libwebrtc::rtp_transceiver::RtpTransceiverInit;
/// Priority for encoding parameters.
pub use libwebrtc::rtp_parameters::Priority;
