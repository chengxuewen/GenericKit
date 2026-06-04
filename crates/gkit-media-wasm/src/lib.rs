//! WASM bindings for gkit-media — WebRTC API for JavaScript via wasm-bindgen.

use js_sys;
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
use gkit_media::capture::generator::{SquarePattern, FramePattern};
use gkit_media::video::buffer::{I420Buffer, VideoBuffer, VideoFormatType};
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::frame::{VideoFrame, BoxVideoFrame};
use gkit_media::video::source_sink::{VideoSink, VideoSource, VideoSinkWants, VideoBroadcaster};

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

    /// Register a sink to receive incoming remote video frames.
    /// The sink's callback will be called with (rgba: Uint8Array, width: number, height: number)
    /// whenever a new frame arrives.
    pub fn add_sink(&self, sink: &RtcVideoSink) {
        let adapter = RtcVideoSinkAdapter {
            callback: sink.callback.clone(),
        };
        self.inner.add_sink(Box::new(adapter));
    }
}

// ─── RtcVideoSource ────────────────────────────────────────────────────

/// Video frame source that generates square-pattern test frames (I420).
/// Call `start()` to begin frame generation, `stop()` to end.
/// Implements VideoSource so it can be passed to `create_video_track`.
#[wasm_bindgen]
pub struct RtcVideoSource {
    broadcaster: Arc<VideoBroadcaster<BoxVideoFrame>>,
    running: Arc<AtomicBool>,
    width: u32,
    height: u32,
    fps: u32,
}

#[wasm_bindgen]
impl RtcVideoSource {
    /// Create a new video source.
    /// width, height: frame dimensions in pixels.
    /// fps: target frames per second.
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        Self {
            broadcaster: Arc::new(VideoBroadcaster::new()),
            running: Arc::new(AtomicBool::new(false)),
            width,
            height,
            fps,
        }
    }

    /// Start frame generation. No-op if already running.
    pub fn start(&self) {
        if self.running.swap(true, Ordering::Relaxed) {
            return;
        }
        let broadcaster = Arc::clone(&self.broadcaster);
        let running = Arc::clone(&self.running);
        let width = self.width;
        let height = self.height;
        let fps = self.fps;
        #[cfg(target_arch = "wasm32")]
        let interval_ms = (1000 / fps) as i32;

        #[cfg(not(target_arch = "wasm32"))]
        {
            #[allow(unused_variables)]
            let _handle = std::thread::spawn(move || {
                let frame_dur = Duration::from_micros(1_000_000 / fps as u64);
                let mut pattern = SquarePattern::new(width, height, 10);
                while running.load(Ordering::Relaxed) {
                    let start_time = std::time::Instant::now();
                    let mut buf = I420Buffer::new(width, height);
                    pattern.draw(
                        &mut buf.data_y, &mut buf.data_u, &mut buf.data_v,
                        buf.stride_y, buf.stride_u, buf.stride_v,
                    );
                    let frame = VideoFrame::new(Box::new(buf) as Box<dyn VideoBuffer>);
                    broadcaster.on_frame(&frame);
                    let elapsed = start_time.elapsed();
                    if elapsed < frame_dur {
                        std::thread::sleep(frame_dur - elapsed);
                    }
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let interval_handle: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
            let ih_clone = interval_handle.clone();
            let running_clone = running.clone();

            let cb = Closure::wrap(Box::new(move || {
                if !running_clone.load(Ordering::Relaxed) {
                    if let Some(id) = *ih_clone.borrow() {
                        web_sys::window().unwrap().clear_interval_with_handle(id);
                    }
                    return;
                }
                // Simple gray I420 buffer: Y=127 (mid-gray), U/V=127 (no color).
                // Avoid I420Buffer::new() + SquarePattern::draw() which panic on wasm32.
                let stride_y = width;
                let stride_u = (width + 1) / 2;
                let stride_v = (width + 1) / 2;
                let y_size = (stride_y * height) as usize;
                let uv_size = (stride_u * ((height + 1) / 2)) as usize;
                let i420 = I420Buffer {
                    width,
                    height,
                    stride_y,
                    stride_u,
                    stride_v,
                    data_y: vec![127u8; y_size],
                    data_u: vec![127u8; uv_size],
                    data_v: vec![127u8; uv_size],
                };
                let frame = VideoFrame::new(Box::new(i420) as Box<dyn VideoBuffer>);
                broadcaster.on_frame(&frame);
            }) as Box<dyn FnMut()>);

            let handle = web_sys::window().unwrap()
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    interval_ms,
                ).unwrap();
            *interval_handle.borrow_mut() = Some(handle);
            cb.forget(); // Keep alive across JS callbacks
        }
    }

    /// Stop frame generation.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Frame width in pixels.
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Frame height in pixels.
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Target frames per second.
    #[wasm_bindgen(getter)]
    pub fn fps(&self) -> u32 {
        self.fps
    }

    /// Add a sink for local visualization of generated frames.
    /// The callback will be called with
    /// `callback(rgba: Uint8Array, width: number, height: number)`
    /// whenever a new frame is generated.
    /// This allows displaying sender-side video without a second PeerConnection.
    pub fn add_sink(&self, callback: js_sys::Function) {
        let adapter = RtcVideoSinkAdapter {
            callback,
        };
        self.broadcaster.add_or_update_sink(
            Box::new(adapter),
            VideoSinkWants { is_active: true, ..Default::default() },
        );
    }
}

/// Delegate VideoSource to the internal broadcaster.
impl VideoSource<BoxVideoFrame> for RtcVideoSource {
    fn add_or_update_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>, wants: VideoSinkWants) {
        self.broadcaster.add_or_update_sink(sink, wants);
    }

    fn remove_sink(&self, sink: &dyn VideoSink<BoxVideoFrame>) {
        self.broadcaster.remove_sink(sink);
    }
}

// ─── RtcVideoSink ─────────────────────────────────────────────────────

/// Receives incoming video frames and calls a JS callback with
/// `callback(rgba: Uint8Array, width: number, height: number)`.
///
/// Usage from JavaScript:
/// ```js
/// const sink = new RtcVideoSink((rgba, w, h) => {
///     // Draw rgba pixels to canvas...
/// });
/// track.addSink(sink);
/// ```
#[wasm_bindgen]
pub struct RtcVideoSink {
    pub(crate) callback: js_sys::Function,
}

#[wasm_bindgen]
impl RtcVideoSink {
    #[wasm_bindgen(constructor)]
    pub fn new(callback: js_sys::Function) -> Self {
        Self { callback }
    }
}

/// Internal adapter: wraps js_sys::Function + I420→RGBA conversion.
/// This is NOT a wasm_bindgen struct, so it can be Box<dyn VideoSink>.
struct RtcVideoSinkAdapter {
    callback: js_sys::Function,
}

// SAFETY: wasm32-unknown-unknown is single-threaded. JS callbacks only
// execute on the main browser thread, so accessing js_sys::Function from
// multiple conceptual "threads" is safe because there is only one real thread.
#[cfg(target_arch = "wasm32")]
unsafe impl Send for RtcVideoSinkAdapter {}

impl VideoSink<BoxVideoFrame> for RtcVideoSinkAdapter {
    fn on_frame(&self, frame: &BoxVideoFrame) {
        if let Some(i420) = frame.buffer.as_i420() {
            let w = i420.width;
            let h = i420.height;
            let rgba_size = (w * h * 4) as usize;
            let mut rgba = vec![0u8; rgba_size];
            i420_to_argb(i420, &mut rgba, w * 4, VideoFormatType::RGBA);
            let array = js_sys::Uint8Array::new_with_length(rgba_size as u32);
            array.copy_from(&rgba);
            let _ = self.callback.call3(
                &JsValue::NULL,
                &array.into(),
                &JsValue::from_f64(w as f64),
                &JsValue::from_f64(h as f64),
            );
        }
    }

    fn on_discarded_frame(&self) {}
}

/// Internal: wraps Arc<VideoBroadcaster> to implement VideoSource so that
/// RtcVideoSource can be shared between create_video_track and local visualization.
struct SharedVideoSource {
    broadcaster: Arc<VideoBroadcaster<BoxVideoFrame>>,
}

impl VideoSource<BoxVideoFrame> for SharedVideoSource {
    fn add_or_update_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>, wants: VideoSinkWants) {
        self.broadcaster.add_or_update_sink(sink, wants);
    }

    fn remove_sink(&self, sink: &dyn VideoSink<BoxVideoFrame>) {
        self.broadcaster.remove_sink(sink);
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

    /// Create a local video track backed by the given source (sender side).
    /// The source remains usable in JS for local visualization via addSink().
    pub fn create_video_track(&self, source: &RtcVideoSource) -> Result<RtcVideoTrack, JsValue> {
        let shared = SharedVideoSource {
            broadcaster: Arc::clone(&source.broadcaster),
        };
        let track = self
            .inner
            .create_video_track(Box::new(shared))
            .map_err(|e| JsValue::from_str(&e.message))?;
        Ok(RtcVideoTrack { inner: track })
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

// Register wasm backend on module load
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
fn register_wasm_rtc_backend() {
    gkit_media::protocols::rtc::peer::RtcEngine::register("wasm", || {
        Box::new(gkit_plugin_webrtc_web_sys::WasmFactory::default())
    });
}

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
