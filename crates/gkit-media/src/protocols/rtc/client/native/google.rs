use crate::protocols::rtc::client::core::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState,
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
    SessionDescription, SignalingState,
};

pub struct GooglePeerConnection {
    state: IceConnectionState,
    closed: bool,
    local_desc: Option<SessionDescription>,
    remote_desc: Option<SessionDescription>,
}

pub struct GoogleDataChannel {
    label: String,
    state: DataChannelState,
    closed: bool,
}

pub struct GoogleFactory;

// --- GooglePeerConnection ---

impl GooglePeerConnection {
    pub fn new() -> MediaResult<Self> {
        Ok(Self {
            state: IceConnectionState::New,
            closed: false,
            local_desc: None,
            remote_desc: None,
        })
    }
}

impl PeerConnection for GooglePeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        Ok(SessionDescription { sdp_type: "offer".into(), sdp: String::new() })
    }
    fn create_answer(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        Ok(SessionDescription { sdp_type: "answer".into(), sdp: String::new() })
    }
    fn set_local_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        self.check_closed()?;
        self.local_desc = Some(desc.clone());
        Ok(())
    }
    fn set_remote_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        self.check_closed()?;
        self.remote_desc = Some(desc.clone());
        Ok(())
    }
    fn add_ice_candidate(&mut self, _candidate: &str, _sdp_mid: &str) -> MediaResult<()> { self.check_closed() }
    fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> {
        self.check_closed()?;
        Ok(Box::new(GoogleDataChannel::new(label)))
    }
    fn ice_connection_state(&self) -> IceConnectionState {
        if self.closed { IceConnectionState::Closed } else { self.state }
    }
    fn connection_state(&self) -> ConnectionState {
        if self.closed { ConnectionState::Closed } else { ConnectionState::New }
    }
    fn gathering_state(&self) -> GatheringState { GatheringState::New }
    fn signaling_state(&self) -> SignalingState { SignalingState::Stable }
    fn local_description(&self) -> MediaResult<SessionDescription> {
        self.local_desc.clone().ok_or_else(|| MediaError::new("no local description"))
    }
    fn remote_description(&self) -> MediaResult<SessionDescription> {
        self.remote_desc.clone().ok_or_else(|| MediaError::new("no remote description"))
    }
    fn close(&mut self) -> MediaResult<()> { self.closed = true; self.state = IceConnectionState::Closed; Ok(()) }
}

impl GooglePeerConnection {
    fn check_closed(&self) -> MediaResult<()> {
        if self.closed { Err(MediaError::new("closed")) } else { Ok(()) }
    }
}

// --- GoogleDataChannel ---

impl GoogleDataChannel {
    pub fn new(label: &str) -> Self {
        Self { label: label.into(), state: DataChannelState::Open, closed: false }
    }
}

impl DataChannel for GoogleDataChannel {
    fn label(&self) -> &str { &self.label }
    fn ready_state(&self) -> DataChannelState {
        if self.closed { DataChannelState::Closed } else { self.state }
    }
    fn send_text(&self, _data: &str) -> MediaResult<()> {
        if self.closed { Err(MediaError::new("closed")) } else { Ok(()) }
    }
    fn send_bytes(&self, _data: &[u8]) -> MediaResult<()> {
        if self.closed { Err(MediaError::new("closed")) } else { Ok(()) }
    }
    fn close(&mut self) -> MediaResult<()> { self.closed = true; self.state = DataChannelState::Closed; Ok(()) }
}

// --- GoogleFactory ---

impl Default for GoogleFactory {
    fn default() -> Self { Self }
}

impl PeerConnectionFactory for GoogleFactory {
    fn backend_name(&self) -> &'static str { "google_lk" }
    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> {
        Ok(Box::new(GooglePeerConnection::new()?))
    }
    fn create_peer_connection_with_config(&self, _c: &RtcConfiguration) -> MediaResult<Box<dyn PeerConnection>> {
        Ok(Box::new(GooglePeerConnection::new()?))
    }
}

#[cfg(feature = "backend-native-google")]
crate::gkit_register_rtc_backend!("google_lk", GoogleFactory);
