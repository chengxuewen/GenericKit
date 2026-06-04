//! WASM bindings for gkit-media — WebRTC API for JavaScript via wasm-bindgen.

use js_sys;
use wasm_bindgen::prelude::*;

// ─── RtcConfiguration ─────────────────────────────────────────────────

#[wasm_bindgen]
pub struct RtcConfiguration {
    #[wasm_bindgen(getter_with_clone)]
    pub ice_servers: Vec<IceServer>,
    #[wasm_bindgen(getter_with_clone)]
    pub ice_transport_policy: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub ice_candidate_pool_size: Option<u32>,
}

#[wasm_bindgen]
impl RtcConfiguration {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            ice_servers: vec![],
            ice_transport_policy: None,
            ice_candidate_pool_size: None,
        }
    }

    pub fn add_ice_server(&mut self, server: IceServer) {
        self.ice_servers.push(server);
    }
}

// ─── IceServer ────────────────────────────────────────────────────────

#[wasm_bindgen]
#[derive(Clone)]
pub struct IceServer {
    #[wasm_bindgen(getter_with_clone)]
    pub urls: Vec<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub username: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub credential: Option<String>,
}

#[wasm_bindgen]
impl IceServer {
    #[wasm_bindgen(constructor)]
    pub fn new(urls: Vec<String>) -> Self {
        Self {
            urls,
            username: None,
            credential: None,
        }
    }
}

// ─── RtcIceCandidate ──────────────────────────────────────────────────

#[wasm_bindgen]
pub struct RtcIceCandidate {
    #[wasm_bindgen(getter_with_clone)]
    pub candidate: String,
    #[wasm_bindgen(getter_with_clone)]
    pub sdp_mid: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub sdp_mline_index: Option<u16>,
}

#[wasm_bindgen]
impl RtcIceCandidate {
    #[wasm_bindgen(constructor)]
    pub fn new(
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    ) -> Self {
        Self {
            candidate,
            sdp_mid,
            sdp_mline_index,
        }
    }
}

// ─── RtcSessionDescription ────────────────────────────────────────────

#[wasm_bindgen]
pub struct RtcSessionDescription {
    #[wasm_bindgen(getter_with_clone)]
    pub sdp_type: String,
    #[wasm_bindgen(getter_with_clone)]
    pub sdp: String,
}

impl From<gkit_media::protocols::rtc::peer::SessionDescription> for RtcSessionDescription {
    fn from(desc: gkit_media::protocols::rtc::peer::SessionDescription) -> Self {
        Self {
            sdp_type: desc.sdp_type,
            sdp: desc.sdp,
        }
    }
}

// ─── RtcDataChannel ───────────────────────────────────────────────────

#[wasm_bindgen]
pub struct RtcDataChannel {
    inner: Box<dyn gkit_media::protocols::rtc::peer::DataChannel>,
}

#[wasm_bindgen]
impl RtcDataChannel {
    pub fn label(&self) -> String {
        self.inner.label().to_string()
    }

    pub fn ready_state(&self) -> String {
        format!("{:?}", self.inner.ready_state())
    }

    pub fn send_text(&self, data: &str) -> Result<(), JsValue> {
        self.inner
            .send_text(data)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn send_bytes(&self, data: &[u8]) -> Result<(), JsValue> {
        self.inner
            .send_bytes(data)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn close(&mut self) -> Result<(), JsValue> {
        self.inner
            .close()
            .map_err(|e| JsValue::from_str(&e.message))
    }
}

// ─── RtcVideoTrack ────────────────────────────────────────────────────

#[wasm_bindgen]
pub struct RtcVideoTrack {
    inner: Box<dyn gkit_media::protocols::rtc::peer::VideoTrack>,
}

#[wasm_bindgen]
impl RtcVideoTrack {
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    pub fn kind(&self) -> String {
        self.inner.kind().to_string()
    }
}

// ─── RtcPeerConnection ────────────────────────────────────────────────

#[wasm_bindgen]
pub struct RtcPeerConnection {
    inner: Box<dyn gkit_media::protocols::rtc::peer::PeerConnection>,
}

#[wasm_bindgen]
impl RtcPeerConnection {
    /// Create a new RtcPeerConnection with default config.
    #[wasm_bindgen(constructor)]
    pub fn new(config: &RtcConfiguration) -> Result<RtcPeerConnection, JsValue> {
        let gkit_config = to_gkit_config(config);
        let factory = gkit_media::protocols::rtc::peer::RtcEngine::create("wasm")
            .map_err(|e| JsValue::from_str(&e.message))?;
        let _factory_ref = factory.as_ref();
        let pc = factory
            .create_peer_connection_with_config(&gkit_config)
            .map_err(|e| JsValue::from_str(&e.message))?;
        Ok(Self { inner: pc })
    }

    pub fn create_offer(&self) -> Result<RtcSessionDescription, JsValue> {
        self.inner
            .create_offer()
            .map(RtcSessionDescription::from)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn create_answer(&self) -> Result<RtcSessionDescription, JsValue> {
        self.inner
            .create_answer()
            .map(RtcSessionDescription::from)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn set_local_description(&mut self, desc: &RtcSessionDescription) -> Result<(), JsValue> {
        let gkit_desc = gkit_media::protocols::rtc::peer::SessionDescription {
            sdp_type: desc.sdp_type.clone(),
            sdp: desc.sdp.clone(),
        };
        self.inner
            .set_local_description(&gkit_desc)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn set_remote_description(&mut self, desc: &RtcSessionDescription) -> Result<(), JsValue> {
        let gkit_desc = gkit_media::protocols::rtc::peer::SessionDescription {
            sdp_type: desc.sdp_type.clone(),
            sdp: desc.sdp.clone(),
        };
        self.inner
            .set_remote_description(&gkit_desc)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn close(&mut self) -> Result<(), JsValue> {
        self.inner
            .close()
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn ice_connection_state(&self) -> String {
        format!("{:?}", self.inner.ice_connection_state())
    }

    pub fn connection_state(&self) -> String {
        format!("{:?}", self.inner.connection_state())
    }

    pub fn add_ice_candidate(&mut self, candidate: &str, sdp_mid: &str) -> Result<(), JsValue> {
        self.inner
            .add_ice_candidate(candidate, sdp_mid)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn create_data_channel(&self, label: &str) -> Result<RtcDataChannel, JsValue> {
        self.inner
            .create_data_channel(label)
            .map(|dc| RtcDataChannel { inner: dc })
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn gathering_state(&self) -> String {
        format!("{:?}", self.inner.gathering_state())
    }

    pub fn signaling_state(&self) -> String {
        format!("{:?}", self.inner.signaling_state())
    }

    pub fn local_description(&self) -> Result<RtcSessionDescription, JsValue> {
        self.inner
            .local_description()
            .map(RtcSessionDescription::from)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn remote_description(&self) -> Result<RtcSessionDescription, JsValue> {
        self.inner
            .remote_description()
            .map(RtcSessionDescription::from)
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn local_address(&self) -> Result<String, JsValue> {
        self.inner
            .local_address()
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn remote_address(&self) -> Result<String, JsValue> {
        self.inner
            .remote_address()
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn max_data_channel_stream(&self) -> Result<u32, JsValue> {
        self.inner
            .max_data_channel_stream()
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn remote_max_message_size(&self) -> Result<usize, JsValue> {
        self.inner
            .remote_max_message_size()
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn get_stats_json(&self) -> Result<String, JsValue> {
        self.inner
            .get_stats_json()
            .map_err(|e| JsValue::from_str(&e.message))
    }

    pub fn set_on_track(&self, callback: js_sys::Function) {
        let cb = move |track: Box<dyn gkit_media::protocols::rtc::peer::VideoTrack>| {
            let js_track = RtcVideoTrack { inner: track };
            let _ = callback.call1(&JsValue::NULL, &JsValue::from(js_track));
        };
        self.inner.set_on_track(Box::new(cb));
    }

    pub fn set_on_ice_candidate(&self, callback: js_sys::Function) {
        let cb = move |candidate: gkit_media::protocols::rtc::peer::IceCandidate| {
            let js_candidate = RtcIceCandidate {
                candidate: candidate.candidate,
                sdp_mid: candidate.sdp_mid,
                sdp_mline_index: candidate.sdp_mline_index,
            };
            let _ = callback.call1(&JsValue::NULL, &JsValue::from(js_candidate));
        };
        self.inner.set_on_ice_candidate(Box::new(cb));
    }

    pub fn set_on_ice_connection_state_change(&self, callback: js_sys::Function) {
        let cb = move |state: gkit_media::protocols::rtc::peer::IceConnectionState| {
            let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&format!("{:?}", state)));
        };
        self.inner.set_on_ice_connection_state_change(Box::new(cb));
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────

fn to_gkit_config(config: &RtcConfiguration) -> gkit_media::protocols::rtc::peer::RtcConfiguration {
    use gkit_media::protocols::rtc::peer::{IceServer as GkIceServer, RtcConfiguration as GkRtcConfig};
    GkRtcConfig {
        ice_servers: config
            .ice_servers
            .iter()
            .map(|s| GkIceServer {
                urls: s.urls.clone(),
                username: s.username.clone(),
                credential: s.credential.clone(),
            })
            .collect(),
        ice_transport_policy: config.ice_transport_policy.clone(),
        ice_candidate_pool_size: config.ice_candidate_pool_size,
    }
}

// ─── Version exports ──────────────────────────────────────────────────

/// Returns the full version string.
#[wasm_bindgen]
pub fn get_version() -> String {
    gkit_core::version::version_string()
}

/// Calls gkit_media::media_hello() and returns a greeting string.
#[wasm_bindgen]
pub fn media_hello() -> String {
    gkit_media::media_hello();
    "media_hello!".to_string()
}
