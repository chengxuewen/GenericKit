use crate::webrtc::client::core::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState,
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
    SessionDescription, SignalingState,
};

pub struct NativePeerConnection {
    state: IceConnectionState,
    closed: bool,
}

pub struct NativeDataChannel {
    label: String,
    state: DataChannelState,
    closed: bool,
}

pub struct NativeFactory;

// --- NativePeerConnection ---

impl NativePeerConnection {
    pub fn new() -> Self {
        Self {
            state: IceConnectionState::New,
            closed: false,
        }
    }
}

impl PeerConnection for NativePeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        self.check_closed()?;
        // TODO: real SDP via webrtc-rs
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
        Ok(Box::new(NativeDataChannel::new(label)))
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

impl NativePeerConnection {
    fn check_closed(&self) -> MediaResult<()> {
        if self.closed {
            Err(MediaError::new("PeerConnection is closed"))
        } else {
            Ok(())
        }
    }
}

// --- NativeDataChannel ---

impl NativeDataChannel {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            state: DataChannelState::Open,
            closed: false,
        }
    }
}

impl DataChannel for NativeDataChannel {
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
        // TODO: real send via webrtc-rs
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

// --- NativeFactory ---

impl NativeFactory {
    pub fn new() -> Self {
        Self
    }
}

impl PeerConnectionFactory for NativeFactory {
    type PC = NativePeerConnection;

    fn create_peer_connection(&self) -> MediaResult<Self::PC> {
        Ok(NativePeerConnection::new())
    }

    fn create_peer_connection_with_config(&self, _config: &RtcConfiguration) -> MediaResult<Self::PC> {
        Ok(NativePeerConnection::new())
    }
}

impl Default for NativeFactory {
    fn default() -> Self {
        Self::new()
    }
}
