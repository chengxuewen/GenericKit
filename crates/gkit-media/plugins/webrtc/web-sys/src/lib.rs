use gkit_media::protocols::rtc::client::core::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState, IceServer,
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
    SessionDescription, SignalingState,
};

pub struct WasmPeerConnection {
    state: IceConnectionState,
    closed: bool,
}

pub struct WasmDataChannel {
    label: String,
    state: DataChannelState,
    closed: bool,
}

pub struct WasmFactory;

// --- WasmPeerConnection ---

impl WasmPeerConnection {
    pub fn new() -> Self {
        Self {
            state: IceConnectionState::New,
            closed: false,
        }
    }
}

impl PeerConnection for WasmPeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        // TODO: call browser RTCPeerConnection.createOffer() via web-sys
        Ok(SessionDescription {
            sdp_type: "offer".into(),
            sdp: String::new(),
        })
    }

    fn create_answer(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        Ok(SessionDescription {
            sdp_type: "answer".into(),
            sdp: String::new(),
        })
    }

    fn set_local_description(&mut self, _desc: &SessionDescription) -> MediaResult<()> {
        self.check_closed()?;
        Ok(())
    }

    fn set_remote_description(&mut self, _desc: &SessionDescription) -> MediaResult<()> {
        self.check_closed()?;
        Ok(())
    }

    fn add_ice_candidate(&mut self, _candidate: &str, _sdp_mid: &str) -> MediaResult<()> {
        self.check_closed()?;
        Ok(())
    }

    fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> {
        self.check_closed()?;
        Ok(Box::new(WasmDataChannel::new(label)))
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        self.state
    }

    fn connection_state(&self) -> ConnectionState {
        if self.closed { ConnectionState::Closed } else { ConnectionState::New }
    }

    fn gathering_state(&self) -> GatheringState {
        GatheringState::New
    }

    fn signaling_state(&self) -> SignalingState {
        SignalingState::Stable
    }

    fn local_description(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        Ok(SessionDescription { sdp_type: String::new(), sdp: String::new() })
    }

    fn remote_description(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        Ok(SessionDescription { sdp_type: String::new(), sdp: String::new() })
    }

    fn close(&mut self) -> MediaResult<()> {
        self.closed = true;
        self.state = IceConnectionState::Closed;
        Ok(())
    }
}

impl WasmPeerConnection {
    fn check_closed(&self) -> MediaResult<()> {
        if self.closed {
            Err(MediaError::new("PeerConnection is closed"))
        } else {
            Ok(())
        }
    }
}

// --- WasmDataChannel ---

impl WasmDataChannel {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            state: DataChannelState::Open,
            closed: false,
        }
    }
}

impl DataChannel for WasmDataChannel {
    fn label(&self) -> &str {
        &self.label
    }

    fn ready_state(&self) -> DataChannelState {
        self.state
    }

    fn send_text(&self, _data: &str) -> MediaResult<()> {
        if self.closed {
            return Err(MediaError::new("DataChannel is closed"));
        }
        Ok(())
    }

    fn send_bytes(&self, _data: &[u8]) -> MediaResult<()> {
        if self.closed {
            return Err(MediaError::new("DataChannel is closed"));
        }
        Ok(())
    }

    fn close(&mut self) -> MediaResult<()> {
        self.closed = true;
        self.state = DataChannelState::Closed;
        Ok(())
    }
}

// --- WasmFactory ---

impl WasmFactory {
    pub fn new() -> Self {
        Self
    }
}

impl PeerConnectionFactory for WasmFactory {
    fn backend_name(&self) -> &'static str { "wasm" }

    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> {
        Ok(Box::new(WasmPeerConnection::new()))
    }

    fn create_peer_connection_with_config(&self, _config: &RtcConfiguration) -> MediaResult<Box<dyn PeerConnection>> {
        Ok(Box::new(WasmPeerConnection::new()))
    }
}

impl Default for WasmFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
gkit_media::gkit_register_rtc_backend!("wasm", WasmFactory);
