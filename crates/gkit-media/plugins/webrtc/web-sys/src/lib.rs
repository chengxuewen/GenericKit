use gkit_media::protocols::rtc::peer::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState,
    MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration,
    SessionDescription, SignalingState, VideoTrack as GkVideoTrack,
};
use gkit_media::video::buffer::{VideoBuffer, VideoFormatType};
use gkit_media::video::convert::{argb_to_i420, i420_to_argb};
use gkit_media::video::frame::{BoxVideoFrame, VideoFrame};
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{atomic::AtomicU64, Arc, Mutex};
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlVideoElement, ImageData, MediaStream,
    MediaStreamTrack, RtcConfiguration as WebRtcConfig, RtcIceCandidate as WebRtcIceCandidate,
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
                    let has_video = desc.sdp.contains("m=video");
                    web_sys::console::log_1(&JsValue::from_str(&format!("[SDP offer] hasVideo={} sdpLen={} firstLine={}",
                        has_video, desc.sdp.len(), desc.sdp.lines().next().unwrap_or(""))));
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
                    let has_video = desc.sdp.contains("m=video");
                    web_sys::console::log_1(&JsValue::from_str(&format!("[SDP answer] hasVideo={} sdpLen={}",
                        has_video, desc.sdp.len())));
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
        // Call the browser API directly — do NOT use spawn_local which may not execute.
        // The returned Promise is dropped; the browser API still executes.
        let _ = self.pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice));
        Ok(())
    }

    fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> {
        let dc = self.pc.create_data_channel(label);
        Ok(Box::new(WasmDataChannel::new(label, dc)))
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        map_ice_state(self.pc.ice_connection_state())
    }

    fn set_on_ice_connection_state_change(&self, cb: Box<dyn Fn(IceConnectionState) + Send>) {
        let pc = self.pc.clone();
        let cb = Arc::new(Mutex::new(cb));
        let on_change = Closure::wrap(Box::new(move || {
            let state = map_ice_state(pc.ice_connection_state());
            if let Ok(cb_guard) = cb.lock() {
                cb_guard(state);
            }
        }) as Box<dyn FnMut()>);
        self.pc
            .set_oniceconnectionstatechange(Some(on_change.as_ref().unchecked_ref()));
        on_change.forget();
    }

    fn set_on_ice_candidate(&self, cb: Box<dyn Fn(gkit_media::protocols::rtc::peer::IceCandidate) + Send>) {
        use gkit_media::protocols::rtc::peer::IceCandidate;
        let cb = Arc::new(Mutex::new(cb));
        let on_candidate = Closure::wrap(Box::new(move |event: web_sys::RtcPeerConnectionIceEvent| {
            if let Some(c) = event.candidate() {
                if let Ok(cb_guard) = cb.lock() {
                    cb_guard(IceCandidate {
                        candidate: c.candidate(),
                        sdp_mid: c.sdp_mid(),
                        sdp_mline_index: c.sdp_m_line_index(),
                    });
                }
            }
        }) as Box<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>);
        self.pc
            .set_onicecandidate(Some(on_candidate.as_ref().unchecked_ref()));
        on_candidate.forget();
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
        source: Box<dyn VideoSource<BoxVideoFrame>>,
    ) -> MediaResult<Box<dyn GkVideoTrack>> {
        let window = web_sys::window().ok_or(MediaError::new("no window"))?;
        let document = window
            .document()
            .ok_or(MediaError::new("no document"))?;
        let offscreen_canvas: HtmlCanvasElement = document
            .create_element("canvas")
            .map_err(|e| MediaError::new(format!("create offscreen canvas: {:?}", e)))?
            .dyn_into()
            .map_err(|_| MediaError::new("failed to cast offscreen canvas"))?;
        offscreen_canvas.set_width(640);
        offscreen_canvas.set_height(480);
        let offscreen_ctx: CanvasRenderingContext2d = offscreen_canvas
            .get_context("2d")
            .map_err(|e| MediaError::new(format!("getContext offscreen: {:?}", e)))?
            .ok_or(MediaError::new("no offscreen 2d context"))?
            .dyn_into()
            .map_err(|_| MediaError::new("failed to cast offscreen context"))?;

        let stream_canvas: HtmlCanvasElement = document
            .create_element("canvas")
            .map_err(|e| MediaError::new(format!("create stream canvas: {:?}", e)))?
            .dyn_into()
            .map_err(|_| MediaError::new("failed to cast stream canvas"))?;
        stream_canvas.set_width(640);
        stream_canvas.set_height(480);
        document
            .body()
            .and_then(|body| body.append_child(&stream_canvas).ok());
        stream_canvas.set_attribute("style", "position:fixed;bottom:0;right:0;width:160px;height:90px;z-index:9998;border:2px solid green;background:#000").ok();
        let stream_ctx: CanvasRenderingContext2d = stream_canvas
            .get_context("2d")
            .map_err(|e| MediaError::new(format!("getContext stream: {:?}", e)))?
            .ok_or(MediaError::new("no stream 2d context"))?
            .dyn_into()
            .map_err(|_| MediaError::new("failed to cast stream context"))?;

        let stream = stream_canvas
            .capture_stream_with_frame_request_rate(30.0)
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

        let adapter = CanvasSinkAdapter {
            offscreen_ctx,
            offscreen_canvas: offscreen_canvas.clone(),
            stream_ctx,
            stream_canvas: stream_canvas.clone(),
            width: Rc::new(RefCell::new(640)),
            height: Rc::new(RefCell::new(480)),
            track: track.clone(),
        };
        source.add_or_update_sink(
            Box::new(adapter),
            VideoSinkWants {
                is_active: true,
                ..Default::default()
            },
        );

        Ok(Box::new(WasmVideoTrack {
            _canvas: stream_canvas,
            _stream: stream,
            track,
        }))
    }

    fn set_on_track(&self, cb: Box<dyn Fn(Box<dyn GkVideoTrack>) + Send>) {
        let cb = Arc::new(Mutex::new(cb));
        let cb_clone = cb.clone();

        let ontrack = Closure::wrap(Box::new(move |event: RtcTrackEvent| {
            let track = event.track();
            // Get the sender's MediaStream from the ontrack event.
            // Using the original stream preserves the track-stream association
            // that the browser's WebRTC stack expects for remote playback.
            let stream = event.streams().get(0);
            let remote_track = WasmRemoteVideoTrack::new(track, stream);
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

/// Sink that receives frames from a VideoSource and draws them onto a canvas.
///
/// Uses a dual-canvas architecture:
/// 1. **Offscreen canvas** (not in DOM) — receives `putImageData` with raw RGBA pixels.
/// 2. **Streaming canvas** (in DOM, `display:none`) — copies from offscreen via `drawImage`,
///    which notifies the browser compositor so `captureStream()` produces frames.
struct CanvasSinkAdapter {
    offscreen_ctx: CanvasRenderingContext2d,
    offscreen_canvas: HtmlCanvasElement,
    stream_ctx: CanvasRenderingContext2d,
    stream_canvas: HtmlCanvasElement,
    width: Rc<RefCell<u32>>,
    height: Rc<RefCell<u32>>,
    track: MediaStreamTrack, 
}

// SAFETY: wasm32-unknown-unknown is single-threaded. CanvasRenderingContext2d
// and HtmlCanvasElement are JsValue wrappers — all JS calls happen on the
// main browser thread. Rc<RefCell> is safe for the same reason.
unsafe impl Send for CanvasSinkAdapter {}

impl VideoSink<BoxVideoFrame> for CanvasSinkAdapter {
    fn on_frame(&self, frame: &BoxVideoFrame) {
        if let Some(i420) = frame.buffer.as_i420() {
            let w = i420.width;
            let h = i420.height;

            {
                let mut cw = self.width.borrow_mut();
                let mut ch = self.height.borrow_mut();
                if *cw != w || *ch != h {
                    *cw = w;
                    *ch = h;
                    self.offscreen_canvas.set_width(w);
                    self.offscreen_canvas.set_height(h);
                    self.stream_canvas.set_width(w);
                    self.stream_canvas.set_height(h);
                }
            }

            let rgba_size = (w * h * 4) as usize;
            let mut rgba = vec![0u8; rgba_size];
            i420_to_argb(i420, &mut rgba, w * 4, VideoFormatType::RGBA);

            let image_data = match ImageData::new_with_u8_clamped_array_and_sh(
                wasm_bindgen::Clamped(&mut rgba),
                w,
                h,
            ) {
                Ok(data) => data,
                Err(_) => return,
            };
            self.offscreen_ctx
                .put_image_data(&image_data, 0.0, 0.0)
                .ok();

            // Step 2: copy offscreen → streaming canvas via drawImage.
            // This notifies the browser compositor, which is required
            // for captureStream() to produce frames.
            self.stream_ctx
                .clear_rect(0.0, 0.0, w as f64, h as f64);
            self.stream_ctx
                .draw_image_with_html_canvas_element(&self.offscreen_canvas, 0.0, 0.0)
                .ok();

            if let Ok(f) = js_sys::Reflect::get(self.track.as_ref(), &JsValue::from_str("requestFrame")) {
                if let Ok(func) = f.dyn_into::<js_sys::Function>() {
                    let _ = func.call0(self.track.as_ref());
                }
            } else {
                static REQ_FAIL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let n = REQ_FAIL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if n <= 3 { web_sys::console::warn_1(&JsValue::from_str(&format!("[TX WARN] requestFrame not found on track (attempt {})", n))); }
            }

            static SENDER_FRAMES: AtomicU64 = AtomicU64::new(0);
            let n = SENDER_FRAMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n <= 5 || n % 30 == 0 {
                web_sys::console::log_1(&JsValue::from_str(
                    &format!("[TX frame] #{}: {}x{} I420 → canvas putImageData → drawImage → requestFrame", n, w, h),
                ));
            }
        }
    }

    fn on_discarded_frame(&self) {}
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

pub struct WasmRemoteVideoTrack {
    track: MediaStreamTrack,
    /// The MediaStream associated with this track from the ontrack event.
    /// Preserving the original stream is important for remote video playback
    /// — browsers expect the track to be played from its original stream context.
    stream: Option<MediaStream>,
    sinks: Arc<Mutex<Vec<Box<dyn VideoSink<BoxVideoFrame>>>>>,
    started: Mutex<bool>,
}

impl WasmRemoteVideoTrack {
    pub fn new(track: MediaStreamTrack, stream: JsValue) -> Self {
        let stream = stream.dyn_into::<MediaStream>().ok();
        Self {
            track,
            stream,
            sinks: Arc::new(Mutex::new(Vec::new())),
            started: Mutex::new(false),
        }
    }

    /// Returns the underlying DOM MediaStreamTrack for JS-side playback.
    pub fn raw_track(&self) -> MediaStreamTrack {
        self.track.clone()
    }
}

// SAFETY: WASM is single-threaded. MediaStreamTrack is a JsValue wrapper.
unsafe impl Send for WasmRemoteVideoTrack {}
unsafe impl Sync for WasmRemoteVideoTrack {}

impl GkVideoTrack for WasmRemoteVideoTrack {
    fn id(&self) -> &str {
        "wasm-remote-video"
    }

    fn kind(&self) -> &str {
        "video"
    }

    #[cfg(target_arch = "wasm32")]
    fn raw_track_js(&self) -> wasm_bindgen::prelude::JsValue {
        wasm_bindgen::prelude::JsValue::from(self.track.clone())
    }

    fn add_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>) {
        {
            let mut sinks = self
                .sinks
                .lock()
                .expect("add_sink: sinks mutex poisoned");
            sinks.push(sink);
        }

        let mut started = self
            .started
            .lock()
            .expect("add_sink: started mutex poisoned");
        if *started {
            return;
        }
        *started = true;
        drop(started);

        let track = self.track.clone();
        let sinks = Arc::clone(&self.sinks);
        let stream_opt = self.stream.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };
            let document = match window.document() {
                Some(d) => d,
                None => return,
            };

            let video: HtmlVideoElement = match document.create_element("video") {
                Ok(el) => match el.dyn_into() {
                    Ok(v) => v,
                    Err(_) => return,
                },
                Err(_) => return,
            };
            video.set_muted(true);
            video.set_autoplay(true);
            video.set_attribute("playsinline", "").ok();
            video.set_attribute("style", "position:fixed;left:-9999px;top:-9999px").ok();
            document
                .body()
                .and_then(|body| body.append_child(&video).ok());

            // Prefer the stream from the ontrack event, which preserves the
            // track-stream association the browser expects for remote playback.
            // Fall back to MediaStream::new_with_tracks([track]) which creates
            // the stream with the track in the constructor (matching standard JS
            // pattern `new MediaStream([track])`).
            let stream = match &stream_opt {
                Some(s) => s.clone(),
                None => {
                    let tracks = js_sys::Array::new();
                    tracks.push(&JsValue::from(track.clone()));
                    match MediaStream::new_with_tracks(&tracks) {
                        Ok(s) => s,
                        Err(_) => return,
                    }
                }
            };

            js_sys::Reflect::set(
                &video,
                &JsValue::from_str("srcObject"),
                &JsValue::from(stream),
            )
            .ok();

            // Explicit play — some browsers need this even with autoplay
            let _ = video.play();

            let canvas: HtmlCanvasElement = match document.create_element("canvas") {
                Ok(el) => match el.dyn_into() {
                    Ok(c) => c,
                    Err(_) => return,
                },
                Err(_) => return,
            };
            // Set canvas size once — avoid clearing canvas every frame
            canvas.set_width(640);
            canvas.set_height(480);

            let ctx: CanvasRenderingContext2d = match canvas.get_context("2d") {
                Ok(Some(c)) => match c.dyn_into() {
                    Ok(ctx) => ctx,
                    Err(_) => return,
                },
                _ => return,
            };

            let frame_count = Rc::new(RefCell::new(0u64));
            let fc = frame_count.clone();

            let cb = Closure::wrap(Box::new(move || {
                let vw = video.video_width();
                let vh = video.video_height();

                {
                    let mut count = fc.borrow_mut();
                    *count += 1;
                    if *count <= 5 || *count % 30 == 0 {
                        let ready_state = MediaStreamTrack::ready_state(&track);
                        web_sys::console::log_1(&JsValue::from_str(
                            &format!(
                                "[RX track] readyState={:?}, loop #{}, videoWidth={}, videoHeight={}",
                                ready_state, *count, vw, vh
                            ),
                        ));
                    }
                }

                if vw == 0 || vh == 0 {
                    return;
                }

                ctx.draw_image_with_html_video_element_and_dw_and_dh(
                    &video,
                    0.0,
                    0.0,
                    vw as f64,
                    vh as f64,
                )
                .ok();

                let img_data = match ctx.get_image_data(0.0, 0.0, vw as f64, vh as f64) {
                    Ok(d) => d,
                    Err(_) => return,
                };
                let rgba = img_data.data().to_vec();

                let i420 = match argb_to_i420(&rgba, vw, vh, vw * 4) {
                    Ok(b) => b,
                    Err(_) => return,
                };

                let frame = VideoFrame::new(Box::new(i420) as Box<dyn VideoBuffer>);

                if let Ok(sinks_guard) = sinks.lock() {
                    for sink in sinks_guard.iter() {
                        sink.on_frame(&frame);
                    }
                }

                {
                    let count = fc.borrow();
                    if *count <= 5 || *count % 30 == 0 {
                        web_sys::console::log_1(&JsValue::from_str(
                            &format!(
                                "[RX frame] #{}, {}x{} extracted",
                                *count, vw, vh
                            ),
                        ));
                    }
                }
            }) as Box<dyn FnMut()>);

            window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    33,
                )
                .ok();
            cb.forget();
        });
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
