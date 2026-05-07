use std::fmt;
use std::sync::Arc;
use crate::video::frame::BoxVideoFrame;
use crate::video::source_sink::{VideoSink, VideoSource};

/// Error type for media operations.
#[derive(Debug)]
pub struct MediaError {
    pub message: String,
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MediaError: {}", self.message)
    }
}

impl std::error::Error for MediaError {}

impl MediaError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

pub type MediaResult<T> = Result<T, MediaError>;

/// ICE connection state as defined by W3C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceConnectionState {
    New,
    Checking,
    Connected,
    Completed,
    Failed,
    Disconnected,
    Closed,
}

/// W3C RTCPeerConnectionState.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

/// W3C RTCIceGatheringState.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatheringState {
    New,
    Gathering,
    Complete,
}

/// W3C RTCSignalingState.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalingState {
    Stable,
    HaveLocalOffer,
    HaveRemoteOffer,
    HaveLocalPranswer,
    HaveRemotePranswer,
}

/// Data channel state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataChannelState {
    Connecting,
    Open,
    Closing,
    Closed,
}

/// RTCSessionDescriptionInit from W3C.
#[derive(Debug, Clone)]
pub struct SessionDescription {
    pub sdp_type: String,
    pub sdp: String,
}

/// ICE server configuration (STUN/TURN).
#[derive(Debug, Clone, Default)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// PeerConnection configuration.
#[derive(Debug, Clone, Default)]
pub struct RtcConfiguration {
    pub ice_servers: Vec<IceServer>,
    pub ice_transport_policy: Option<String>, // "all" or "relay"
    pub ice_candidate_pool_size: Option<u32>,
}

pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u16>,
}

/// W3C MediaStreamTrack-like abstraction for video.
/// Sender: created via `create_video_track(source)`, bridges VideoSource → RTP.
/// Receiver: obtained via `set_on_track` callback, `add_sink()` registers consumers.
pub trait VideoTrack: Send {
    fn id(&self) -> &str;
    fn kind(&self) -> &str;
    fn add_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>);
}

/// W3C RTCPeerConnection trait.
pub trait PeerConnection: Send {
    fn create_offer(&self) -> MediaResult<SessionDescription>;
    fn create_answer(&self) -> MediaResult<SessionDescription>;
    fn set_local_description(&mut self, desc: &SessionDescription) -> MediaResult<()>;
    fn set_remote_description(&mut self, desc: &SessionDescription) -> MediaResult<()>;
    fn add_ice_candidate(&mut self, candidate: &str, sdp_mid: &str) -> MediaResult<()>;
    fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>>;
    fn ice_connection_state(&self) -> IceConnectionState;
    fn connection_state(&self) -> ConnectionState;
    fn gathering_state(&self) -> GatheringState;
    fn signaling_state(&self) -> SignalingState;
    fn local_description(&self) -> MediaResult<SessionDescription>;
    fn remote_description(&self) -> MediaResult<SessionDescription>;
    fn local_address(&self) -> MediaResult<String> { Err(MediaError::new("not available")) }
    fn remote_address(&self) -> MediaResult<String> { Err(MediaError::new("not available")) }
    fn max_data_channel_stream(&self) -> MediaResult<u32> { Ok(0) }
    fn remote_max_message_size(&self) -> MediaResult<usize> { Ok(65536) }
    fn close(&mut self) -> MediaResult<()>;

    /// Create a local video track backed by the given source (sender side).
    fn create_video_track(&self, _source: Box<dyn VideoSource<BoxVideoFrame>>) -> MediaResult<Box<dyn VideoTrack>> { Err(MediaError::new("not supported")) }
    /// Register callback for incoming remote tracks (receiver side).
    fn set_on_track(&self, _cb: Box<dyn Fn(Box<dyn VideoTrack>) + Send>) {}

    /// Register callback for local ICE candidates.
    fn set_on_ice_candidate(&self, _cb: Box<dyn Fn(IceCandidate) + Send>) {}
    /// Register callback for ICE connection state changes.
    fn set_on_ice_connection_state_change(&self, _cb: Box<dyn Fn(IceConnectionState) + Send>) {}
    /// Wait until ICE gathering is complete.
    fn gather_complete(&self) -> MediaResult<()> { Ok(()) }
}

/// W3C RTCDataChannel trait.
pub trait DataChannel: Send {
    fn label(&self) -> &str;
    fn ready_state(&self) -> DataChannelState;
    fn send_text(&self, data: &str) -> MediaResult<()>;
    fn send_bytes(&self, data: &[u8]) -> MediaResult<()>;
    fn stream_id(&self) -> MediaResult<u32> { Err(MediaError::new("not available")) }
    fn protocol(&self) -> MediaResult<String> { Err(MediaError::new("not available")) }
    fn close(&mut self) -> MediaResult<()>;
}

/// Factory trait for backend creation.
pub trait PeerConnectionFactory {
    type PC: PeerConnection;

    fn create_peer_connection(&self) -> MediaResult<Self::PC>;
    fn create_peer_connection_with_config(&self, config: &RtcConfiguration) -> MediaResult<Self::PC>;
}
