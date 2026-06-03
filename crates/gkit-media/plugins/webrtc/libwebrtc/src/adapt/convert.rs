use gkit_media::protocols::rtc::peer::core::{
    ConnectionState, GatheringState, IceConnectionState, SessionDescription, SignalingState,
};

pub fn lk_conn_state(s: libwebrtc::peer_connection::PeerConnectionState) -> ConnectionState {
    use libwebrtc::peer_connection::PeerConnectionState as Lk;
    match s {
        Lk::New => ConnectionState::New,
        Lk::Connecting => ConnectionState::Connecting,
        Lk::Connected => ConnectionState::Connected,
        Lk::Disconnected => ConnectionState::Disconnected,
        Lk::Failed => ConnectionState::Failed,
        Lk::Closed => ConnectionState::Closed,
    }
}

pub fn lk_ice_state(s: libwebrtc::peer_connection::IceConnectionState) -> IceConnectionState {
    use libwebrtc::peer_connection::IceConnectionState as Lk;
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

pub fn lk_gathering_state(s: libwebrtc::peer_connection::IceGatheringState) -> GatheringState {
    use libwebrtc::peer_connection::IceGatheringState as Lk;
    match s {
        Lk::New => GatheringState::New,
        Lk::Gathering => GatheringState::Gathering,
        Lk::Complete => GatheringState::Complete,
    }
}

pub fn lk_signaling_state(s: libwebrtc::peer_connection::SignalingState) -> SignalingState {
    use libwebrtc::peer_connection::SignalingState as Lk;
    match s {
        Lk::Stable => SignalingState::Stable,
        Lk::HaveLocalOffer => SignalingState::HaveLocalOffer,
        Lk::HaveLocalPrAnswer => SignalingState::HaveLocalPranswer,
        Lk::HaveRemoteOffer => SignalingState::HaveRemoteOffer,
        Lk::HaveRemotePrAnswer => SignalingState::HaveRemotePranswer,
        Lk::Closed => SignalingState::Stable,
    }
}

pub fn lk_sdp_to_core(
    sd: libwebrtc::session_description::SessionDescription,
) -> SessionDescription {
    SessionDescription {
        sdp_type: sd.sdp_type().to_string(),
        sdp: sd.to_string(),
    }
}

pub fn lk_ice_candidate_to_core(
    ic: libwebrtc::ice_candidate::IceCandidate,
) -> gkit_media::protocols::rtc::peer::core::IceCandidate {
    gkit_media::protocols::rtc::peer::core::IceCandidate {
        candidate: ic.candidate(),
        sdp_mid: {
            let mid = ic.sdp_mid();
            if mid.is_empty() { None } else { Some(mid) }
        },
        sdp_mline_index: {
            let idx = ic.sdp_mline_index();
            if idx < 0 { None } else { Some(idx as u16) }
        },
    }
}

pub fn lk_dc_state(
    s: libwebrtc::data_channel::DataChannelState,
) -> gkit_media::protocols::rtc::peer::core::DataChannelState {
    use gkit_media::protocols::rtc::peer::core::DataChannelState as G;
    use libwebrtc::data_channel::DataChannelState as L;
    match s {
        L::Connecting => G::Connecting,
        L::Open => G::Open,
        L::Closing => G::Closing,
        L::Closed => G::Closed,
    }
}

pub fn gkit_rotation_to_lk(
    r: gkit_media::video::frame::VideoRotation,
) -> libwebrtc::video_frame::VideoRotation {
    use gkit_media::video::frame::VideoRotation as G;
    use libwebrtc::video_frame::VideoRotation as L;
    match r {
        G::Rotation0 => L::VideoRotation0,
        G::Rotation90 => L::VideoRotation90,
        G::Rotation180 => L::VideoRotation180,
        G::Rotation270 => L::VideoRotation270,
    }
}

pub fn lk_rotation_to_gkit(
    r: libwebrtc::video_frame::VideoRotation,
) -> gkit_media::video::frame::VideoRotation {
    use gkit_media::video::frame::VideoRotation as G;
    use libwebrtc::video_frame::VideoRotation as L;
    match r {
        L::VideoRotation0 => G::Rotation0,
        L::VideoRotation90 => G::Rotation90,
        L::VideoRotation180 => G::Rotation180,
        L::VideoRotation270 => G::Rotation270,
    }
}
