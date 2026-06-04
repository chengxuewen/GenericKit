use gkit_media::protocols::rtc::peer::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState,
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
    SessionDescription, SignalingState, VideoTrack as GkVideoTrack,
};
use gkit_media::video::frame::BoxVideoFrame;
use gkit_media::video::source_sink::{VideoSink, VideoSource};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, MediaStream, MediaStreamTrack,
    RtcConfiguration as WebRtcConfig, RtcIceCandidate as WebRtcIceCandidate,
    RtcIceCandidateInit, RtcPeerConnection as WebRtcPeerConnection, RtcSdpType,
    RtcSessionDescriptionInit, RtcTrackEvent,
};

// ─── WasmPeerConnection ────────────────────────────────────────────────

pub struct WasmPeerConnection {
    pc: WebRtcPeerConnection,
    local_desc: Rc<RefCell<Option<SessionDescription>>>,
    remote_desc: Rc<RefCell<Option<SessionDescription>>>,
    stats: Rc<RefCell<Option<String>>>,
}

// SAFETY: WASM is single-threaded, so Rc<RefCell> is effectively thread-safe.
// WebRtcPeerConnection (JsValue) is already Send+Sync on wasm32.
unsafe impl Send for WasmPeerConnection {}
unsafe impl Sync for WasmPeerConnection {}

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
        Ok(Self {
            pc,
            local_desc: Rc::new(RefCell::new(None)),
            remote_desc: Rc::new(RefCell::new(None)),
            stats: Rc::new(RefCell::new(None)),
        })
    }

    pub fn new_default() -> MediaResult<Self> {
        Self::new(&RtcConfiguration::default())
    }

}

impl PeerConnection for WasmPeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        let pc = self.pc.clone();
        let local = self.local_desc.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let promise = pc.create_offer();
            if let Ok(js_val) = JsFuture::from(promise).await {
                let desc = extract_session_description(&js_val);
                if let Ok(ref desc) = desc {
                    let init = RtcSessionDescriptionInit::new(map_sdp_type_str(&desc.sdp_type));
                    init.set_sdp(&desc.sdp);
                    let _ = JsFuture::from(pc.set_local_description(&init)).await;
                    *local.borrow_mut() = Some(desc.clone());
                }
            }
        });
        Ok(self
            .local_desc
            .borrow()
            .clone()
            .unwrap_or(SessionDescription {
                sdp_type: "offer".into(),
                sdp: String::new(),
            }))
    }

    fn create_answer(&self) -> MediaResult<SessionDescription> {
        let pc = self.pc.clone();
        let local = self.local_desc.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let promise = pc.create_answer();
            if let Ok(js_val) = JsFuture::from(promise).await {
                let desc = extract_session_description(&js_val);
                if let Ok(ref desc) = desc {
                    let init = RtcSessionDescriptionInit::new(map_sdp_type_str(&desc.sdp_type));
                    init.set_sdp(&desc.sdp);
                    let _ = JsFuture::from(pc.set_local_description(&init)).await;
                    *local.borrow_mut() = Some(desc.clone());
                }
            }
        });
        Ok(self
            .local_desc
            .borrow()
            .clone()
            .unwrap_or(SessionDescription {
                sdp_type: "answer".into(),
                sdp: String::new(),
            }))
    }

    fn set_local_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        let init = RtcSessionDescriptionInit::new(map_sdp_type_str(&desc.sdp_type));
        init.set_sdp(&desc.sdp);
        let pc = self.pc.clone();
        let local = self.local_desc.clone();
        let desc_clone = desc.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let promise = pc.set_local_description(&init);
            let _ = JsFuture::from(promise).await;
            *local.borrow_mut() = Some(desc_clone);
        });
        Ok(())
    }

    fn set_remote_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        let init = RtcSessionDescriptionInit::new(map_sdp_type_str(&desc.sdp_type));
        init.set_sdp(&desc.sdp);
        let pc = self.pc.clone();
        let remote = self.remote_desc.clone();
        let desc_clone = desc.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let promise = pc.set_remote_description(&init);
            let _ = JsFuture::from(promise).await;
            *remote.borrow_mut() = Some(desc_clone);
        });
        Ok(())
    }

    fn add_ice_candidate(&mut self, candidate: &str, sdp_mid: &str) -> MediaResult<()> {
        let init = RtcIceCandidateInit::new(candidate);
        if !sdp_mid.is_empty() {
            init.set_sdp_mid(Some(sdp_mid));
        }
        let ice = WebRtcIceCandidate::new(&init)
            .map_err(|e| MediaError::new(format!("RtcIceCandidate: {:?}", e)))?;
        let pc = self.pc.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let promise = pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice));
            let _ = JsFuture::from(promise).await;
        });
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
        self.local_desc
            .borrow()
            .clone()
            .ok_or_else(|| MediaError::new("no local description"))
    }

    fn remote_description(&self) -> MediaResult<SessionDescription> {
        self.remote_desc
            .borrow()
            .clone()
            .ok_or_else(|| MediaError::new("no remote description"))
    }

    fn create_video_track(
        &self,
        _source: Box<dyn VideoSource<BoxVideoFrame>>,
    ) -> MediaResult<Box<dyn GkVideoTrack>> {
        let window = web_sys::window().ok_or(MediaError::new("no window"))?;
        let document = window
            .document()
            .ok_or(MediaError::new("no document"))?;
        let canvas: HtmlCanvasElement = document
            .create_element("canvas")
            .map_err(|e| MediaError::new(format!("create canvas: {:?}", e)))?
            .dyn_into()
            .map_err(|_| MediaError::new("failed to cast to canvas"))?;
        canvas.set_width(640);
        canvas.set_height(480);
        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .map_err(|e| MediaError::new(format!("getContext: {:?}", e)))?
            .ok_or(MediaError::new("no 2d context"))?
            .dyn_into()
            .map_err(|_| MediaError::new("failed to cast context"))?;

        let stream = canvas
            .capture_stream()
            .map_err(|_| MediaError::new("canvas.captureStream() failed"))?;
        let video_tracks = stream.get_video_tracks();
        let track = video_tracks.get(0);
        if track.is_undefined() {
            return Err(MediaError::new("no video track from canvas"));
        }
        let track: MediaStreamTrack = track
            .dyn_into()
            .map_err(|_| MediaError::new("failed to cast video track"))?;

        let _ = self.pc.add_track_0(&track, &stream);

        // Draw a test pattern so the canvas stream has initial content
        ctx.set_fill_style_str("green");
        ctx.fill_rect(0.0, 0.0, 640.0, 480.0);

        Ok(Box::new(WasmVideoTrack {
            _canvas: canvas,
            _stream: stream,
            track,
        }))
    }

    fn set_on_track(&self, cb: Box<dyn Fn(Box<dyn GkVideoTrack>) + Send>) {
        let cb = Arc::new(Mutex::new(cb));
        let cb_clone = cb.clone();

        let ontrack = Closure::wrap(Box::new(move |event: RtcTrackEvent| {
            let track = event.track();
            let remote_track = WasmRemoteVideoTrack { track };
            let cb_guard = cb_clone
                .lock()
                .expect("set_on_track callback mutex poisoned");
            cb_guard(Box::new(remote_track));
        }) as Box<dyn FnMut(RtcTrackEvent)>);

        self.pc.set_ontrack(Some(ontrack.as_ref().unchecked_ref()));
        ontrack.forget();
    }

    fn close(&mut self) -> MediaResult<()> {
        self.pc.close();
        Ok(())
    }

    fn get_stats_json(&self) -> MediaResult<String> {
        let pc = self.pc.clone();
        let stats_cache = self.stats.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let promise = pc.get_stats();
            if let Ok(stats) = JsFuture::from(promise).await {
                let json = js_sys::JSON::stringify(&stats);
                if let Ok(json_str) = json {
                    *stats_cache.borrow_mut() = json_str.as_string();
                }
            }
        });
        self.stats
            .borrow()
            .clone()
            .ok_or_else(|| MediaError::new("stats not yet available"))
    }
}

// ─── WasmVideoTrack (sender) ────────────────────────────────────────────

#[allow(dead_code)]
pub struct WasmVideoTrack {
    _canvas: HtmlCanvasElement,
    _stream: MediaStream,
    track: MediaStreamTrack,
}

impl GkVideoTrack for WasmVideoTrack {
    fn id(&self) -> &str {
        "wasm-video"
    }

    fn kind(&self) -> &str {
        "video"
    }

    fn add_sink(&self, _sink: Box<dyn VideoSink<BoxVideoFrame>>) {
        // Sender track: sinks are not used; frames come from canvas capture
    }
}

// ─── WasmRemoteVideoTrack (receiver) ────────────────────────────────────

#[allow(dead_code)]
pub struct WasmRemoteVideoTrack {
    track: MediaStreamTrack,
}

impl GkVideoTrack for WasmRemoteVideoTrack {
    fn id(&self) -> &str {
        "wasm-remote-video"
    }

    fn kind(&self) -> &str {
        "video"
    }

    fn add_sink(&self, _sink: Box<dyn VideoSink<BoxVideoFrame>>) {
        // TODO: Spawn a frame reader: video → canvas → getImageData → sink.on_frame()
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
