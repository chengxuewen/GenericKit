use crate::protocols::rtc::client::core::{
    ConnectionState, DataChannel, GatheringState,
    IceConnectionState, MediaResult, PeerConnection,
    SessionDescription, SignalingState,
};

impl From<libwebrtc::peer_connection::PeerConnectionState> for ConnectionState {
    fn from(s: libwebrtc::peer_connection::PeerConnectionState) -> Self {
        match s {
            libwebrtc::peer_connection::PeerConnectionState::New => ConnectionState::New,
            libwebrtc::peer_connection::PeerConnectionState::Connecting => ConnectionState::Connecting,
            libwebrtc::peer_connection::PeerConnectionState::Connected => ConnectionState::Connected,
            libwebrtc::peer_connection::PeerConnectionState::Disconnected => ConnectionState::Disconnected,
            libwebrtc::peer_connection::PeerConnectionState::Failed => ConnectionState::Failed,
            libwebrtc::peer_connection::PeerConnectionState::Closed => ConnectionState::Closed,
        }
    }
}

impl From<libwebrtc::peer_connection::IceConnectionState> for IceConnectionState {
    fn from(s: libwebrtc::peer_connection::IceConnectionState) -> Self {
        match s {
            libwebrtc::peer_connection::IceConnectionState::New => IceConnectionState::New,
            libwebrtc::peer_connection::IceConnectionState::Checking => IceConnectionState::Checking,
            libwebrtc::peer_connection::IceConnectionState::Connected => IceConnectionState::Connected,
            libwebrtc::peer_connection::IceConnectionState::Completed => IceConnectionState::Completed,
            libwebrtc::peer_connection::IceConnectionState::Failed => IceConnectionState::Failed,
            libwebrtc::peer_connection::IceConnectionState::Disconnected => IceConnectionState::Disconnected,
            libwebrtc::peer_connection::IceConnectionState::Closed => IceConnectionState::Closed,
            libwebrtc::peer_connection::IceConnectionState::Max => IceConnectionState::Closed,
        }
    }
}

impl From<libwebrtc::peer_connection::IceGatheringState> for GatheringState {
    fn from(s: libwebrtc::peer_connection::IceGatheringState) -> Self {
        match s {
            libwebrtc::peer_connection::IceGatheringState::New => GatheringState::New,
            libwebrtc::peer_connection::IceGatheringState::Gathering => GatheringState::Gathering,
            libwebrtc::peer_connection::IceGatheringState::Complete => GatheringState::Complete,
        }
    }
}

impl From<libwebrtc::peer_connection::SignalingState> for SignalingState {
    fn from(s: libwebrtc::peer_connection::SignalingState) -> Self {
        match s {
            libwebrtc::peer_connection::SignalingState::Stable => SignalingState::Stable,
            libwebrtc::peer_connection::SignalingState::HaveLocalOffer => SignalingState::HaveLocalOffer,
            libwebrtc::peer_connection::SignalingState::HaveLocalPrAnswer => SignalingState::HaveLocalPranswer,
            libwebrtc::peer_connection::SignalingState::HaveRemoteOffer => SignalingState::HaveRemoteOffer,
            libwebrtc::peer_connection::SignalingState::HaveRemotePrAnswer => SignalingState::HaveRemotePranswer,
            libwebrtc::peer_connection::SignalingState::Closed => SignalingState::Stable,
        }
    }
}

pub struct LiveKitPeerConnection {
    inner: libwebrtc::peer_connection::PeerConnection,
}

impl LiveKitPeerConnection {
    pub fn new(pc: libwebrtc::peer_connection::PeerConnection) -> Self {
        Self { inner: pc }
    }
}

impl PeerConnection for LiveKitPeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        todo!()
    }

    fn create_answer(&self) -> MediaResult<SessionDescription> {
        todo!()
    }

    fn set_local_description(&mut self, _desc: &SessionDescription) -> MediaResult<()> {
        todo!()
    }

    fn set_remote_description(&mut self, _desc: &SessionDescription) -> MediaResult<()> {
        todo!()
    }

    fn add_ice_candidate(&mut self, _candidate: &str, _sdp_mid: &str) -> MediaResult<()> {
        todo!()
    }

    fn create_data_channel(&self, _label: &str) -> MediaResult<Box<dyn DataChannel>> {
        todo!()
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        self.inner.ice_connection_state().into()
    }

    fn connection_state(&self) -> ConnectionState {
        self.inner.connection_state().into()
    }

    fn gathering_state(&self) -> GatheringState {
        self.inner.ice_gathering_state().into()
    }

    fn signaling_state(&self) -> SignalingState {
        self.inner.signaling_state().into()
    }

    fn local_description(&self) -> MediaResult<SessionDescription> {
        todo!()
    }

    fn remote_description(&self) -> MediaResult<SessionDescription> {
        todo!()
    }

    fn close(&mut self) -> MediaResult<()> {
        self.inner.close();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_state_mapping() {
        use libwebrtc::peer_connection::PeerConnectionState as LkState;
        let cases = vec![
            (LkState::New, ConnectionState::New),
            (LkState::Connecting, ConnectionState::Connecting),
            (LkState::Connected, ConnectionState::Connected),
            (LkState::Disconnected, ConnectionState::Disconnected),
            (LkState::Failed, ConnectionState::Failed),
            (LkState::Closed, ConnectionState::Closed),
        ];
        for (lk, expected) in cases {
            assert_eq!(ConnectionState::from(lk), expected);
        }
    }

    #[test]
    fn ice_connection_state_mapping() {
        use libwebrtc::peer_connection::IceConnectionState as LkState;
        assert_eq!(IceConnectionState::from(LkState::New), IceConnectionState::New);
        assert_eq!(IceConnectionState::from(LkState::Checking), IceConnectionState::Checking);
        assert_eq!(IceConnectionState::from(LkState::Connected), IceConnectionState::Connected);
        assert_eq!(IceConnectionState::from(LkState::Completed), IceConnectionState::Completed);
        assert_eq!(IceConnectionState::from(LkState::Failed), IceConnectionState::Failed);
        assert_eq!(IceConnectionState::from(LkState::Disconnected), IceConnectionState::Disconnected);
        assert_eq!(IceConnectionState::from(LkState::Closed), IceConnectionState::Closed);
        assert_eq!(IceConnectionState::from(LkState::Max), IceConnectionState::Closed);
    }

    #[test]
    fn gathering_state_mapping() {
        use libwebrtc::peer_connection::IceGatheringState as LkState;
        assert_eq!(GatheringState::from(LkState::New), GatheringState::New);
        assert_eq!(GatheringState::from(LkState::Gathering), GatheringState::Gathering);
        assert_eq!(GatheringState::from(LkState::Complete), GatheringState::Complete);
    }

    #[test]
    fn signaling_state_mapping() {
        use libwebrtc::peer_connection::SignalingState as LkState;
        assert_eq!(SignalingState::from(LkState::Stable), SignalingState::Stable);
        assert_eq!(SignalingState::from(LkState::HaveLocalOffer), SignalingState::HaveLocalOffer);
        assert_eq!(SignalingState::from(LkState::HaveLocalPrAnswer), SignalingState::HaveLocalPranswer);
        assert_eq!(SignalingState::from(LkState::HaveRemoteOffer), SignalingState::HaveRemoteOffer);
        assert_eq!(SignalingState::from(LkState::HaveRemotePrAnswer), SignalingState::HaveRemotePranswer);
        assert_eq!(SignalingState::from(LkState::Closed), SignalingState::Stable);
    }
}
