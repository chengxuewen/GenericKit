use std::sync::OnceLock;

use libwebrtc::peer_connection_factory::PeerConnectionFactory as LkPcf;
use libwebrtc::peer_connection_factory::RtcConfiguration as LkRtcConfig;

use gkit_media::protocols::rtc::peer::core::{
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
};

use crate::adapt::peer_connection::LiveKitPeerConnection;

static PCF: OnceLock<LkPcf> = OnceLock::new();

pub(crate) fn get_pcf() -> &'static LkPcf {
    // Ensure the tokio runtime and global handle are initialized before
    // creating the libwebrtc PeerConnectionFactory (which spawns C++ threads
    // that call livekit_runtime::spawn()).
    crate::adapt::peer_connection::rt();
    PCF.get_or_init(|| LkPcf::default())
}

pub struct LiveKitRsFactory;

impl Default for LiveKitRsFactory {
    fn default() -> Self {
        Self
    }
}

impl LiveKitRsFactory {
    pub fn new() -> Self {
        // Force rt() initialization early — set_handle() must be called
        // before libwebrtc creates its internal C++ threads, which call
        // livekit_runtime::spawn() from any thread.
        crate::adapt::peer_connection::rt();
        Self
    }
}

impl PeerConnectionFactory for LiveKitRsFactory {
    fn backend_name(&self) -> &'static str {
        "google"
    }

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
