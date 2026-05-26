use std::sync::OnceLock;

use libwebrtc::peer_connection_factory::PeerConnectionFactory as LkPcf;
use libwebrtc::peer_connection_factory::RtcConfiguration as LkRtcConfig;

use gkit_media::protocols::rtc::client::core::{
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
};

use crate::adapt::peer_connection::LiveKitPeerConnection;

/// Global libwebrtc PeerConnectionFactory — libwebrtc requires exactly one per process.
static PCF: OnceLock<LkPcf> = OnceLock::new();

pub(crate) fn get_pcf() -> &'static LkPcf {
    PCF.get_or_init(|| LkPcf::default())
}

pub struct LiveKitRsFactory;

impl Default for LiveKitRsFactory {
    fn default() -> Self { Self }
}

impl LiveKitRsFactory {
    pub fn new() -> Self { Self }
}

impl PeerConnectionFactory for LiveKitRsFactory {
    fn backend_name(&self) -> &'static str { "google" }

    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> {
        self.create_peer_connection_with_config(&RtcConfiguration::default())
    }

    fn create_peer_connection_with_config(
        &self,
        _config: &RtcConfiguration,
    ) -> MediaResult<Box<dyn PeerConnection>> {
        let lk_config = LkRtcConfig::default();
        let pc = get_pcf()
            .create_peer_connection(lk_config)
            .map_err(|e| MediaError::new(format!("create PC: {e}")))?;
        Ok(Box::new(LiveKitPeerConnection::new(pc)))
    }
}
