use std::sync::OnceLock;

use super::google_lk as lk;

use crate::protocols::rtc::client::core::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceCandidate,
    IceConnectionState, IceServer, MediaError, MediaResult, PeerConnection,
    PeerConnectionFactory, RtcConfiguration, SessionDescription, SignalingState,
};

// ============================================================================
// GooglePeerConnection
// ============================================================================

pub struct GooglePeerConnection {
    inner: lk::peer_connection::PeerConnection,
    rt: &'static tokio::runtime::Runtime,
}

pub struct GoogleDataChannel {
    inner: lk::data_channel::DataChannel,
    label: String,
}

pub struct GoogleFactory {
    inner: lk::peer_connection_factory::PeerConnectionFactory,
    rt: &'static tokio::runtime::Runtime,
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    })
}

fn to_media_error(e: lk::RtcError) -> MediaError {
    MediaError::new(format!("google_lk: {:?} - {}", e.error_type, e.message))
}

// --- SessionDescription conversion ---

fn to_lk_sd(desc: &SessionDescription) -> MediaResult<lk::session_description::SessionDescription> {
    use lk::session_description::SdpType;
    let sdp_type = match desc.sdp_type.as_str() {
        "offer" => SdpType::Offer,
        "answer" => SdpType::Answer,
        "pranswer" => SdpType::PrAnswer,
        "rollback" => SdpType::Rollback,
        _ => return Err(MediaError::new(format!("unknown sdp_type: {}", desc.sdp_type))),
    };
    Ok(lk::session_description::SessionDescription { sdp_type, sdp: desc.sdp.clone() })
}

fn from_lk_sd(desc: lk::session_description::SessionDescription) -> SessionDescription {
    use lk::session_description::SdpType;
    let sdp_type = match desc.sdp_type {
        SdpType::Offer => "offer",
        SdpType::Answer => "answer",
        SdpType::PrAnswer => "pranswer",
        SdpType::Rollback => "rollback",
    };
    SessionDescription { sdp_type: sdp_type.to_string(), sdp: desc.sdp }
}

// --- RtcConfiguration conversion ---

fn to_lk_config(config: &RtcConfiguration) -> lk::peer_connection_factory::RtcConfiguration {
    let ice_servers: Vec<lk::peer_connection_factory::IceServer> = config
        .ice_servers
        .iter()
        .map(|s| lk::peer_connection_factory::IceServer {
            urls: s.urls.clone(),
            username: s.username.clone().unwrap_or_default(),
            password: s.credential.clone().unwrap_or_default(),
        })
        .collect();
    lk::peer_connection_factory::RtcConfiguration {
        ice_servers,
        ..Default::default()
    }
}

// --- Enum mappings ---

fn map_conn_state(s: lk::peer_connection::PeerConnectionState) -> ConnectionState {
    use lk::peer_connection::PeerConnectionState as Lk;
    match s {
        Lk::New => ConnectionState::New,
        Lk::Connecting => ConnectionState::Connecting,
        Lk::Connected => ConnectionState::Connected,
        Lk::Disconnected => ConnectionState::Disconnected,
        Lk::Failed => ConnectionState::Failed,
        Lk::Closed => ConnectionState::Closed,
    }
}

fn map_ice_state(s: lk::peer_connection::IceConnectionState) -> IceConnectionState {
    use lk::peer_connection::IceConnectionState as Lk;
    match s {
        Lk::New => IceConnectionState::New,
        Lk::Checking => IceConnectionState::Checking,
        Lk::Connected => IceConnectionState::Connected,
        Lk::Completed => IceConnectionState::Completed,
        Lk::Failed => IceConnectionState::Failed,
        Lk::Disconnected => IceConnectionState::Disconnected,
        Lk::Closed => IceConnectionState::Closed,
        Lk::Max => IceConnectionState::Closed,
    }
}

fn map_gather_state(s: lk::peer_connection::IceGatheringState) -> GatheringState {
    use lk::peer_connection::IceGatheringState as Lk;
    match s {
        Lk::New => GatheringState::New,
        Lk::Gathering => GatheringState::Gathering,
        Lk::Complete => GatheringState::Complete,
    }
}

fn map_signaling_state(s: lk::peer_connection::SignalingState) -> SignalingState {
    use lk::peer_connection::SignalingState as Lk;
    match s {
        Lk::Stable => SignalingState::Stable,
        Lk::HaveLocalOffer => SignalingState::HaveLocalOffer,
        Lk::HaveLocalPrAnswer => SignalingState::HaveLocalPranswer,
        Lk::HaveRemoteOffer => SignalingState::HaveRemoteOffer,
        Lk::HaveRemotePrAnswer => SignalingState::HaveRemotePranswer,
        Lk::Closed => SignalingState::Stable,
    }
}

fn map_dc_state(s: lk::data_channel::DataChannelState) -> DataChannelState {
    use lk::data_channel::DataChannelState as Lk;
    match s {
        Lk::Connecting => DataChannelState::Connecting,
        Lk::Open => DataChannelState::Open,
        Lk::Closing => DataChannelState::Closing,
        Lk::Closed => DataChannelState::Closed,
    }
}

// ============================================================================
// GooglePeerConnection impl
// ============================================================================

impl GooglePeerConnection {
    pub fn new(config: &RtcConfiguration) -> MediaResult<Self> {
        let factory = lk::peer_connection_factory::PeerConnectionFactory::default();
        let pc = factory
            .create_peer_connection(to_lk_config(config))
            .map_err(to_media_error)?;
        Ok(Self {
            inner: pc,
            rt: runtime(),
        })
    }
}

impl PeerConnection for GooglePeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        self.rt.block_on(async {
            let desc = self
                .inner
                .create_offer(lk::peer_connection::OfferOptions::default())
                .await
                .map_err(to_media_error)?;
            Ok(from_lk_sd(desc))
        })
    }

    fn create_answer(&self) -> MediaResult<SessionDescription> {
        self.rt.block_on(async {
            let desc = self
                .inner
                .create_answer(lk::peer_connection::AnswerOptions::default())
                .await
                .map_err(to_media_error)?;
            Ok(from_lk_sd(desc))
        })
    }

    fn set_local_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        if desc.sdp.is_empty() {
            return Ok(());
        }
        self.rt.block_on(async {
            self.inner
                .set_local_description(to_lk_sd(desc)?)
                .await
                .map_err(to_media_error)
        })
    }

    fn set_remote_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        if desc.sdp.is_empty() {
            return Ok(());
        }
        self.rt.block_on(async {
            self.inner
                .set_remote_description(to_lk_sd(desc)?)
                .await
                .map_err(to_media_error)
        })
    }

    fn add_ice_candidate(&mut self, candidate: &str, sdp_mid: &str) -> MediaResult<()> {
        if candidate.is_empty() {
            return Ok(());
        }
        let ice = lk::ice_candidate::IceCandidate::parse(sdp_mid, 0, candidate)
            .map_err(|_| MediaError::new("failed to parse ICE candidate"))?;
        self.rt.block_on(async {
            self.inner
                .add_ice_candidate(ice)
                .await
                .map_err(to_media_error)
        })
    }

    fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> {
        let dc = self
            .inner
            .create_data_channel(label, lk::data_channel::DataChannelInit::default())
            .map_err(to_media_error)?;
        Ok(Box::new(GoogleDataChannel {
            label: label.to_string(),
            inner: dc,
        }))
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        map_ice_state(self.inner.ice_connection_state())
    }

    fn connection_state(&self) -> ConnectionState {
        map_conn_state(self.inner.connection_state())
    }

    fn gathering_state(&self) -> GatheringState {
        map_gather_state(self.inner.ice_gathering_state())
    }

    fn signaling_state(&self) -> SignalingState {
        map_signaling_state(self.inner.signaling_state())
    }

    fn local_description(&self) -> MediaResult<SessionDescription> {
        self.inner
            .current_local_description()
            .map(from_lk_sd)
            .ok_or_else(|| MediaError::new("no local description"))
    }

    fn remote_description(&self) -> MediaResult<SessionDescription> {
        self.inner
            .current_remote_description()
            .map(from_lk_sd)
            .ok_or_else(|| MediaError::new("no remote description"))
    }

    fn close(&mut self) -> MediaResult<()> {
        self.inner.close();
        Ok(())
    }
}

// ============================================================================
// GoogleDataChannel impl
// ============================================================================

impl DataChannel for GoogleDataChannel {
    fn label(&self) -> &str {
        &self.label
    }

    fn ready_state(&self) -> DataChannelState {
        map_dc_state(self.inner.state())
    }

    fn send_text(&self, data: &str) -> MediaResult<()> {
        self.inner
            .send(data.as_bytes(), false)
            .map_err(|e| MediaError::new(format!("google_lk dc send: {e}")))
    }

    fn send_bytes(&self, data: &[u8]) -> MediaResult<()> {
        self.inner
            .send(data, true)
            .map_err(|e| MediaError::new(format!("google_lk dc send: {e}")))
    }

    fn close(&mut self) -> MediaResult<()> {
        self.inner.close();
        Ok(())
    }
}

// ============================================================================
// GoogleFactory impl
// ============================================================================

impl Default for GoogleFactory {
    fn default() -> Self {
        Self {
            inner: lk::peer_connection_factory::PeerConnectionFactory::default(),
            rt: runtime(),
        }
    }
}

impl PeerConnectionFactory for GoogleFactory {
    fn backend_name(&self) -> &'static str {
        "google_lk"
    }

    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> {
        let pc = self
            .inner
            .create_peer_connection(lk::peer_connection_factory::RtcConfiguration::default())
            .map_err(to_media_error)?;
        Ok(Box::new(GooglePeerConnection {
            inner: pc,
            rt: self.rt,
        }))
    }

    fn create_peer_connection_with_config(
        &self,
        config: &RtcConfiguration,
    ) -> MediaResult<Box<dyn PeerConnection>> {
        let pc = self
            .inner
            .create_peer_connection(to_lk_config(config))
            .map_err(to_media_error)?;
        Ok(Box::new(GooglePeerConnection {
            inner: pc,
            rt: self.rt,
        }))
    }
}

// ============================================================================
// Static registration
// ============================================================================

#[cfg(feature = "backend-native-google")]
gkit_register_rtc_backend!("google_lk", GoogleFactory);
