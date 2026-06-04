//! WASM bindings for gkit-media — WebRTC API for JavaScript via wasm-bindgen.

use wasm_bindgen::prelude::*;

// ─── RtcConfiguration ─────────────────────────────────────────────────

#[wasm_bindgen]
pub struct RtcConfiguration {
    #[wasm_bindgen(getter_with_clone)]
    pub ice_servers: Vec<IceServer>,
}

#[wasm_bindgen]
impl RtcConfiguration {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            ice_servers: vec![],
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
        ..Default::default()
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
