#[cfg(feature = "backend-native-webrtc-rs")]
use std::sync::{Arc, OnceLock};

#[cfg(feature = "backend-native-webrtc-rs")]
use crate::protocols::rtc::client::core::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState,
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
    SessionDescription, SignalingState,
};

#[cfg(feature = "backend-native-webrtc-rs")]
use webrtc::{
    api::APIBuilder,
    peer_connection::{
        configuration::RTCConfiguration as WrtcConfig,
        sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    ice_transport::ice_candidate::RTCIceCandidateInit,
};

#[cfg(feature = "backend-native-webrtc-rs")]
fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap())
}

#[cfg(feature = "backend-native-webrtc-rs")]
pub struct NativePeerConnection {
    pc: Arc<RTCPeerConnection>,
}

#[cfg(feature = "backend-native-webrtc-rs")]
pub struct NativeDataChannel {
    dc: Arc<webrtc::data_channel::RTCDataChannel>,
}

#[cfg(feature = "backend-native-webrtc-rs")]
pub struct NativeFactory {
    pub sync_mode: bool,
}

#[cfg(feature = "backend-native-webrtc-rs")]
impl Default for NativeFactory {
    fn default() -> Self { Self { sync_mode: false } }
}

#[cfg(feature = "backend-native-webrtc-rs")]
impl NativeFactory {
    pub fn new() -> Self { Self::default() }
    pub fn with_sync_mode(sync: bool) -> Self { Self { sync_mode: sync } }
}

#[cfg(feature = "backend-native-webrtc-rs")]
impl NativePeerConnection {
    pub fn new() -> MediaResult<Self> {
        rt().block_on(async {
            let api = APIBuilder::new().build();
            let pc = api.new_peer_connection(WrtcConfig::default()).await
                .map_err(|e| MediaError::new(format!("{e}")))?;
            Ok(Self { pc: Arc::new(pc) })
        })
    }
    fn check_closed(&self) -> MediaResult<()> {
        use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState as S;
        if matches!(self.pc.connection_state(), S::Closed) {
            Err(MediaError::new("closed"))
        } else { Ok(()) }
    }
    fn to_sd(desc: &RTCSessionDescription) -> SessionDescription {
        SessionDescription { sdp_type: "offer".into(), sdp: desc.sdp.clone() } // simplified
    }
}

#[cfg(feature = "backend-native-webrtc-rs")]
impl PeerConnection for NativePeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        rt().block_on(async {
            let o = self.pc.create_offer(None).await.map_err(|e| MediaError::new(format!("{e}")))?;
            Ok(SessionDescription { sdp_type: "offer".into(), sdp: o.sdp })
        })
    }
    fn create_answer(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        rt().block_on(async {
            let a = self.pc.create_answer(None).await.map_err(|e| MediaError::new(format!("{e}")))?;
            Ok(SessionDescription { sdp_type: "answer".into(), sdp: a.sdp })
        })
    }
    fn set_local_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        self.check_closed()?;
        rt().block_on(async {
            let sd = RTCSessionDescription::offer(desc.sdp.clone()).map_err(|e| MediaError::new(format!("{e}")))?;
            self.pc.set_local_description(sd).await.map_err(|e| MediaError::new(format!("{e}")))
        })
    }
    fn set_remote_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        self.check_closed()?;
        rt().block_on(async {
            let sd = RTCSessionDescription::offer(desc.sdp.clone()).map_err(|e| MediaError::new(format!("{e}")))?;
            self.pc.set_remote_description(sd).await.map_err(|e| MediaError::new(format!("{e}")))
        })
    }
    fn add_ice_candidate(&mut self, candidate: &str, sdp_mid: &str) -> MediaResult<()> {
        self.check_closed()?;
        rt().block_on(async {
            self.pc.add_ice_candidate(RTCIceCandidateInit {
                candidate: candidate.to_string(), sdp_mid: Some(sdp_mid.to_string()),
                sdp_mline_index: Some(0), username_fragment: None,
            }).await.map_err(|e| MediaError::new(format!("{e}")))
        })
    }
    fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> {
        self.check_closed()?;
        rt().block_on(async {
            let dc = self.pc.create_data_channel(label, None).await
                .map_err(|e| MediaError::new(format!("{e}")))?;
            Ok(Box::new(NativeDataChannel { dc }) as Box<dyn DataChannel>)
        })
    }
    fn ice_connection_state(&self) -> IceConnectionState {
        use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState as W;
        match self.pc.ice_connection_state() {
            W::New => IceConnectionState::New, W::Checking => IceConnectionState::Checking,
            W::Connected => IceConnectionState::Connected, W::Completed => IceConnectionState::Completed,
            W::Failed => IceConnectionState::Failed, W::Disconnected => IceConnectionState::Disconnected,
            W::Closed => IceConnectionState::Closed, _ => IceConnectionState::New,
        }
    }
    fn connection_state(&self) -> ConnectionState {
        use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState as W;
        match self.pc.connection_state() {
            W::New => ConnectionState::New, W::Connecting => ConnectionState::Connecting,
            W::Connected => ConnectionState::Connected, W::Disconnected => ConnectionState::Disconnected,
            W::Failed => ConnectionState::Failed, W::Closed => ConnectionState::Closed, _ => ConnectionState::New,
        }
    }
    fn gathering_state(&self) -> GatheringState {
        use webrtc::ice_transport::ice_gathering_state::RTCIceGatheringState as W;
        match self.pc.ice_gathering_state() {
            W::New => GatheringState::New, W::Gathering => GatheringState::Gathering,
            W::Complete => GatheringState::Complete, _ => GatheringState::New,
        }
    }
    fn signaling_state(&self) -> SignalingState {
        use webrtc::peer_connection::signaling_state::RTCSignalingState as W;
        match self.pc.signaling_state() {
            W::Stable => SignalingState::Stable, W::HaveLocalOffer => SignalingState::HaveLocalOffer,
            W::HaveLocalPranswer => SignalingState::HaveLocalPranswer, W::HaveRemoteOffer => SignalingState::HaveRemoteOffer,
            W::HaveRemotePranswer => SignalingState::HaveRemotePranswer, _ => SignalingState::Stable,
        }
    }
    fn local_description(&self) -> MediaResult<SessionDescription> {
        rt().block_on(async {
            self.pc.local_description().await
                .map(|d| Self::to_sd(&d)).ok_or_else(|| MediaError::new("no local desc"))
        })
    }
    fn remote_description(&self) -> MediaResult<SessionDescription> {
        rt().block_on(async {
            self.pc.remote_description().await
                .map(|d| Self::to_sd(&d)).ok_or_else(|| MediaError::new("no remote desc"))
        })
    }
    fn close(&mut self) -> MediaResult<()> {
        rt().block_on(async { self.pc.close().await.map_err(|e| MediaError::new(format!("{e}"))) })
    }
}

#[cfg(feature = "backend-native-webrtc-rs")]
impl DataChannel for NativeDataChannel {
    fn label(&self) -> &str { "" }
    fn ready_state(&self) -> DataChannelState {
        use webrtc::data_channel::data_channel_state::RTCDataChannelState as W;
        match self.dc.ready_state() { W::Open => DataChannelState::Open, W::Closed => DataChannelState::Closed, _ => DataChannelState::Connecting }
    }
    fn send_text(&self, data: &str) -> MediaResult<()> {
        rt().block_on(async { self.dc.send_text(data).await.map(|_| ()).map_err(|e| MediaError::new(format!("{e}"))) })
    }
    fn send_bytes(&self, data: &[u8]) -> MediaResult<()> {
        rt().block_on(async { self.dc.send(&bytes::Bytes::copy_from_slice(data)).await.map(|_| ()).map_err(|e| MediaError::new(format!("{e}"))) })
    }
    fn close(&mut self) -> MediaResult<()> {
        rt().block_on(async { self.dc.close().await.map_err(|e| MediaError::new(format!("{e}"))) })
    }
}

#[cfg(feature = "backend-native-webrtc-rs")]
impl PeerConnectionFactory for NativeFactory {
    type PC = NativePeerConnection;
    fn create_peer_connection(&self) -> MediaResult<Self::PC> { NativePeerConnection::new() }
    fn create_peer_connection_with_config(&self, _c: &RtcConfiguration) -> MediaResult<Self::PC> { NativePeerConnection::new() }
}

#[cfg(feature = "backend-native-webrtc-rs")]
impl NativeDataChannel {
    pub fn new(_label: &str) -> Self {
        panic!("NativeDataChannel must be created via PeerConnection::create_data_channel")
    }
}
