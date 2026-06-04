use gkit_media::protocols::rtc::peer::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState,
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
    SessionDescription, SignalingState,
};
use pollster::block_on;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    RtcConfiguration as WebRtcConfig, RtcIceCandidate as WebRtcIceCandidate,
    RtcIceCandidateInit, RtcPeerConnection as WebRtcPeerConnection, RtcSdpType,
    RtcSessionDescriptionInit,
};

// ─── WasmPeerConnection ────────────────────────────────────────────────

pub struct WasmPeerConnection {
    pc: WebRtcPeerConnection,
}

impl WasmPeerConnection {
    pub fn new(config: &RtcConfiguration) -> MediaResult<Self> {
        let js_config = WebRtcConfig::new();
        let ice_servers = js_sys::Array::new();
        for server in &config.ice_servers {
            let js_server = js_sys::Object::new();
            let urls = js_sys::Array::new();
            for url in &server.urls {
                urls.push(&JsValue::from_str(url));
            }
            js_sys::Reflect::set(&js_server, &JsValue::from_str("urls"), &urls).unwrap();
            if let Some(ref u) = server.username {
                js_sys::Reflect::set(
                    &js_server,
                    &JsValue::from_str("username"),
                    &JsValue::from_str(u),
                )
                .unwrap();
            }
            if let Some(ref c) = server.credential {
                js_sys::Reflect::set(
                    &js_server,
                    &JsValue::from_str("credential"),
                    &JsValue::from_str(c),
                )
                .unwrap();
            }
            ice_servers.push(&js_server);
        }
        js_config.set_ice_servers(&ice_servers);

        let pc = WebRtcPeerConnection::new_with_configuration(&js_config)
            .map_err(|e| MediaError::new(format!("RTCPeerConnection: {:?}", e)))?;
        Ok(Self { pc })
    }

    pub fn new_default() -> MediaResult<Self> {
        Self::new(&RtcConfiguration::default())
    }
}

impl PeerConnection for WasmPeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        let promise = self.pc.create_offer();
        let result = block_on(JsFuture::from(promise))
            .map_err(|e| MediaError::new(format!("create_offer future: {:?}", e)))?;
        extract_session_description(&result)
    }

    fn create_answer(&self) -> MediaResult<SessionDescription> {
        let promise = self.pc.create_answer();
        let result = block_on(JsFuture::from(promise))
            .map_err(|e| MediaError::new(format!("create_answer future: {:?}", e)))?;
        extract_session_description(&result)
    }

    fn set_local_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        let init = RtcSessionDescriptionInit::new(map_sdp_type_str(&desc.sdp_type));
        init.set_sdp(&desc.sdp);
        let promise = self.pc.set_local_description(&init);
        let _ = block_on(JsFuture::from(promise))
            .map_err(|e| MediaError::new(format!("set_local_description future: {:?}", e)))?;
        Ok(())
    }

    fn set_remote_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        let init = RtcSessionDescriptionInit::new(map_sdp_type_str(&desc.sdp_type));
        init.set_sdp(&desc.sdp);
        let promise = self.pc.set_remote_description(&init);
        let _ = block_on(JsFuture::from(promise))
            .map_err(|e| MediaError::new(format!("set_remote_description future: {:?}", e)))?;
        Ok(())
    }

    fn add_ice_candidate(&mut self, candidate: &str, sdp_mid: &str) -> MediaResult<()> {
        let init = RtcIceCandidateInit::new(candidate);
        if !sdp_mid.is_empty() {
            init.set_sdp_mid(Some(sdp_mid));
        }
        let ice = WebRtcIceCandidate::new(&init)
            .map_err(|e| MediaError::new(format!("RtcIceCandidate: {:?}", e)))?;
        let promise = self
            .pc
            .add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice));
        let _ = block_on(JsFuture::from(promise))
            .map_err(|e| MediaError::new(format!("add_ice_candidate future: {:?}", e)))?;
        Ok(())
    }

    fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> {
        let dc = self.pc.create_data_channel(label);
        Ok(Box::new(WasmDataChannel::new(label, dc)))
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        map_ice_state(self.pc.ice_connection_state())
    }

    fn connection_state(&self) -> ConnectionState {
        map_connection_state(self.pc.connection_state())
    }

    fn gathering_state(&self) -> GatheringState {
        map_gathering_state(self.pc.ice_gathering_state())
    }

    fn signaling_state(&self) -> SignalingState {
        map_signaling_state(self.pc.signaling_state())
    }

    fn local_description(&self) -> MediaResult<SessionDescription> {
        let js_value =
            js_sys::Reflect::get(&self.pc, &JsValue::from_str("localDescription"))
                .map_err(|e| MediaError::new(format!("localDescription: {:?}", e)))?;
        if js_value.is_null() || js_value.is_undefined() {
            return Err(MediaError::new("no local description"));
        }
        extract_session_description(&js_value)
    }

    fn remote_description(&self) -> MediaResult<SessionDescription> {
        let js_value =
            js_sys::Reflect::get(&self.pc, &JsValue::from_str("remoteDescription"))
                .map_err(|e| MediaError::new(format!("remoteDescription: {:?}", e)))?;
        if js_value.is_null() || js_value.is_undefined() {
            return Err(MediaError::new("no remote description"));
        }
        extract_session_description(&js_value)
    }

    fn close(&mut self) -> MediaResult<()> {
        self.pc.close();
        Ok(())
    }

    fn get_stats_json(&self) -> MediaResult<String> {
        let promise = self.pc.get_stats();
        let stats = block_on(JsFuture::from(promise))
            .map_err(|e| MediaError::new(format!("get_stats future: {:?}", e)))?;
        let json = js_sys::JSON::stringify(&stats)
            .map_err(|e| MediaError::new(format!("JSON.stringify: {:?}", e)))?;
        Ok(json.as_string().unwrap_or_default())
    }
}

// ─── WasmDataChannel ───────────────────────────────────────────────────

pub struct WasmDataChannel {
    dc: web_sys::RtcDataChannel,
    label: String,
}

impl WasmDataChannel {
    pub fn new(label: &str, dc: web_sys::RtcDataChannel) -> Self {
        Self {
            label: label.into(),
            dc,
        }
    }
}

impl DataChannel for WasmDataChannel {
    fn label(&self) -> &str {
        &self.label
    }

    fn ready_state(&self) -> DataChannelState {
        map_dc_state(self.dc.ready_state())
    }

    fn send_text(&self, data: &str) -> MediaResult<()> {
        self.dc
            .send_with_str(data)
            .map_err(|e| MediaError::new(format!("send_text: {:?}", e)))
    }

    fn send_bytes(&self, data: &[u8]) -> MediaResult<()> {
        self.dc
            .send_with_u8_array(data)
            .map_err(|e| MediaError::new(format!("send_bytes: {:?}", e)))
    }

    fn close(&mut self) -> MediaResult<()> {
        self.dc.close();
        Ok(())
    }
}

// ─── WasmFactory ───────────────────────────────────────────────────────

pub struct WasmFactory;

impl WasmFactory {
    pub fn new() -> Self {
        Self
    }
}

impl PeerConnectionFactory for WasmFactory {
    fn backend_name(&self) -> &'static str {
        "wasm"
    }

    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> {
        Ok(Box::new(WasmPeerConnection::new_default()?))
    }

    fn create_peer_connection_with_config(
        &self,
        config: &RtcConfiguration,
    ) -> MediaResult<Box<dyn PeerConnection>> {
        Ok(Box::new(WasmPeerConnection::new(config)?))
    }
}

impl Default for WasmFactory {
    fn default() -> Self {
        Self::new()
    }
}

// ─── State Mapping Helpers ─────────────────────────────────────────────

fn map_ice_state(s: web_sys::RtcIceConnectionState) -> IceConnectionState {
    use web_sys::RtcIceConnectionState::*;
    match s {
        New => IceConnectionState::New,
        Checking => IceConnectionState::Checking,
        Connected => IceConnectionState::Connected,
        Completed => IceConnectionState::Completed,
        Failed => IceConnectionState::Failed,
        Disconnected => IceConnectionState::Disconnected,
        Closed => IceConnectionState::Closed,
        _ => IceConnectionState::New,
    }
}

fn map_connection_state(s: web_sys::RtcPeerConnectionState) -> ConnectionState {
    use web_sys::RtcPeerConnectionState::*;
    match s {
        New => ConnectionState::New,
        Connecting => ConnectionState::Connecting,
        Connected => ConnectionState::Connected,
        Disconnected => ConnectionState::Disconnected,
        Failed => ConnectionState::Failed,
        Closed => ConnectionState::Closed,
        _ => ConnectionState::New,
    }
}

fn map_gathering_state(s: web_sys::RtcIceGatheringState) -> GatheringState {
    use web_sys::RtcIceGatheringState::*;
    match s {
        New => GatheringState::New,
        Gathering => GatheringState::Gathering,
        Complete => GatheringState::Complete,
        _ => GatheringState::New,
    }
}

fn map_signaling_state(s: web_sys::RtcSignalingState) -> SignalingState {
    use web_sys::RtcSignalingState::*;
    match s {
        Stable => SignalingState::Stable,
        HaveLocalOffer => SignalingState::HaveLocalOffer,
        HaveRemoteOffer => SignalingState::HaveRemoteOffer,
        HaveLocalPranswer => SignalingState::HaveLocalPranswer,
        HaveRemotePranswer => SignalingState::HaveRemotePranswer,
        Closed => SignalingState::Stable,
        _ => SignalingState::Stable,
    }
}

fn map_dc_state(s: web_sys::RtcDataChannelState) -> DataChannelState {
    use web_sys::RtcDataChannelState::*;
    match s {
        Connecting => DataChannelState::Connecting,
        Open => DataChannelState::Open,
        Closing => DataChannelState::Closing,
        Closed => DataChannelState::Closed,
        _ => DataChannelState::Closed,
    }
}

fn map_sdp_type_str(s: &str) -> RtcSdpType {
    match s {
        "offer" => RtcSdpType::Offer,
        "pranswer" => RtcSdpType::Pranswer,
        "answer" => RtcSdpType::Answer,
        "rollback" => RtcSdpType::Rollback,
        _ => RtcSdpType::Offer,
    }
}

fn extract_session_description(js_value: &JsValue) -> MediaResult<SessionDescription> {
    let sdp_type = js_sys::Reflect::get(js_value, &JsValue::from_str("type"))
        .map(|v| v.as_string().unwrap_or_default())
        .unwrap_or_default();
    let sdp = js_sys::Reflect::get(js_value, &JsValue::from_str("sdp"))
        .map(|v| v.as_string().unwrap_or_default())
        .unwrap_or_default();
    Ok(SessionDescription { sdp_type, sdp })
}

// ─── WASM Registration ─────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
fn register_wasm_backend() {
    gkit_media::protocols::rtc::peer::RtcEngine::register("wasm", || {
        Box::new(WasmFactory::default())
    });
}
