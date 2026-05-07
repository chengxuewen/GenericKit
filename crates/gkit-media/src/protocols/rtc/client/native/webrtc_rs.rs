#[cfg(feature = "backend-native-webrtc-rs")]
include!("webrtc_rs_impl.rs");

#[cfg(not(feature = "backend-native-webrtc-rs"))]
mod stub {
    use crate::protocols::rtc::client::core::{
        ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState,
        MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
        SessionDescription, SignalingState,
    };

    pub struct NativePeerConnection { pub state: IceConnectionState, pub closed: bool }
    pub struct NativeDataChannel { pub label: String, pub state: DataChannelState, pub closed: bool }
    pub struct NativeFactory { pub sync_mode: bool }

    impl Default for NativeFactory { fn default() -> Self { Self { sync_mode: false } } }
    impl NativeFactory { pub fn new() -> Self { Self::default() } pub fn with_sync_mode(s: bool) -> Self { Self { sync_mode: s } } }

    impl NativeDataChannel { pub fn new(label: &str) -> Self { Self { label: label.into(), state: DataChannelState::Open, closed: false } } }
    impl NativePeerConnection { pub fn new() -> MediaResult<Self> { Ok(Self { state: IceConnectionState::New, closed: false }) } fn check_closed(&self) -> MediaResult<()> { if self.closed { Err(MediaError::new("closed")) } else { Ok(()) } } }

    impl PeerConnection for NativePeerConnection {
        fn create_offer(&self) -> MediaResult<SessionDescription> { self.check_closed()?; Ok(SessionDescription { sdp_type: "offer".into(), sdp: String::new() }) }
        fn create_answer(&self) -> MediaResult<SessionDescription> { self.check_closed()?; Ok(SessionDescription { sdp_type: "answer".into(), sdp: String::new() }) }
        fn set_local_description(&mut self, _d: &SessionDescription) -> MediaResult<()> { self.check_closed() }
        fn set_remote_description(&mut self, _d: &SessionDescription) -> MediaResult<()> { self.check_closed() }
        fn add_ice_candidate(&mut self, _c: &str, _m: &str) -> MediaResult<()> { self.check_closed() }
        fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> { self.check_closed()?; Ok(Box::new(NativeDataChannel::new(label))) }
        fn ice_connection_state(&self) -> IceConnectionState { if self.closed { IceConnectionState::Closed } else { self.state } }
        fn connection_state(&self) -> ConnectionState { if self.closed { ConnectionState::Closed } else { ConnectionState::New } }
        fn gathering_state(&self) -> GatheringState { GatheringState::New }
        fn signaling_state(&self) -> SignalingState { SignalingState::Stable }
        fn local_description(&self) -> MediaResult<SessionDescription> { Err(MediaError::new("stub")) }
        fn remote_description(&self) -> MediaResult<SessionDescription> { Err(MediaError::new("stub")) }
        fn close(&mut self) -> MediaResult<()> { self.closed = true; Ok(()) }
        fn add_track(&self, _t: std::sync::Arc<crate::protocols::rtc::client::core::VideoTrack>) -> MediaResult<()> { Ok(()) }
    }

    impl DataChannel for NativeDataChannel {
        fn label(&self) -> &str { &self.label }
        fn ready_state(&self) -> DataChannelState { if self.closed { DataChannelState::Closed } else { self.state } }
        fn send_text(&self, _d: &str) -> MediaResult<()> { if self.closed { Err(MediaError::new("closed")) } else { Ok(()) } }
        fn send_bytes(&self, _d: &[u8]) -> MediaResult<()> { if self.closed { Err(MediaError::new("closed")) } else { Ok(()) } }
        fn close(&mut self) -> MediaResult<()> { self.closed = true; Ok(()) }
    }

    impl PeerConnectionFactory for NativeFactory {
        type PC = NativePeerConnection;
        fn create_peer_connection(&self) -> MediaResult<Self::PC> { NativePeerConnection::new() }
        fn create_peer_connection_with_config(&self, _c: &RtcConfiguration) -> MediaResult<Self::PC> { NativePeerConnection::new() }
    }
}
#[cfg(not(feature = "backend-native-webrtc-rs"))]
pub use stub::*;
