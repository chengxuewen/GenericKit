// ============================================================================
// Real webrtc-rs backend (feature = "backend-native-webrtc-rs")
// ============================================================================

#[cfg(feature = "backend-native-webrtc-rs")]
mod real {
    use std::sync::{Arc, OnceLock};
    use crate::protocols::rtc::client::core::{
        ConnectionState, DataChannel, DataChannelState, GatheringState, IceCandidate,
        IceConnectionState, MediaError, MediaResult, PeerConnection, PeerConnectionFactory,
        RtcConfiguration, SessionDescription, SignalingState, VideoTrack,
    };
    use webrtc::{
        api::APIBuilder,
        peer_connection::{
            configuration::RTCConfiguration as WrtcConfig,
            sdp::session_description::RTCSessionDescription,
            RTCPeerConnection,
        },
        ice_transport::ice_candidate::RTCIceCandidateInit,
    };

    pub struct NativePeerConnection {
        pc: Arc<RTCPeerConnection>,
    }

    pub struct NativeDataChannel {
        dc: Arc<webrtc::data_channel::RTCDataChannel>,
    }

    pub struct NativeFactory { pub sync_mode: bool }
    impl Default for NativeFactory { fn default() -> Self { Self { sync_mode: false } } }
    impl NativeFactory {
        pub fn new() -> Self { Self::default() }
        pub fn with_sync_mode(sync: bool) -> Self { Self { sync_mode: sync } }
    }

    fn rt() -> &'static tokio::runtime::Runtime {
        static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
        RT.get_or_init(|| tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap())
    }

    impl NativePeerConnection {
        pub fn new() -> MediaResult<Self> {
            Self::with_setting_engine(None)
        }
        pub fn with_setting_engine(se: Option<webrtc::api::setting_engine::SettingEngine>) -> MediaResult<Self> {
            rt().block_on(async {
                let mut m = webrtc::api::media_engine::MediaEngine::default();
                m.register_default_codecs().map_err(|e| MediaError::new(format!("register codecs: {e}")))?;
                let mut builder = APIBuilder::new().with_media_engine(m);
                if let Some(se) = se { builder = builder.with_setting_engine(se); }
                let api = builder.build();
                let pc = api.new_peer_connection(WrtcConfig {
                    ice_servers: vec![webrtc::ice_transport::ice_server::RTCIceServer {
                        urls: vec![
                            "stun:stun.l.google.com:19302".to_string(),
                            "stun:stun.qq.com:3478".to_string(),
                        ],
                        ..Default::default()
                    }],
                    ..Default::default()
                }).await.map_err(|e| MediaError::new(format!("{e}")))?;
                Ok(Self { pc: Arc::new(pc) })
            })
        }
        fn check_closed(&self) -> MediaResult<()> {
            use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState as S;
            if matches!(self.pc.connection_state(), S::Closed) { Err(MediaError::new("closed")) } else { Ok(()) }
        }
    }

    fn make_sd(desc: &SessionDescription) -> MediaResult<RTCSessionDescription> {
        if desc.sdp_type == "answer" {
            RTCSessionDescription::answer(desc.sdp.clone()).map_err(|e| MediaError::new(format!("{e}")))
        } else {
            RTCSessionDescription::offer(desc.sdp.clone()).map_err(|e| MediaError::new(format!("{e}")))
        }
    }

    impl PeerConnection for NativePeerConnection {
        fn create_offer(&self) -> MediaResult<SessionDescription> { self.check_closed()?; rt().block_on(async { let o = self.pc.create_offer(None).await.map_err(|e| MediaError::new(format!("{e}")))?; Ok(SessionDescription { sdp_type: "offer".into(), sdp: o.sdp }) }) }
        fn create_answer(&self) -> MediaResult<SessionDescription> { self.check_closed()?; rt().block_on(async { match self.pc.create_answer(None).await { Ok(a) => Ok(SessionDescription { sdp_type: "answer".into(), sdp: a.sdp }), Err(_) => Ok(SessionDescription { sdp_type: "answer".into(), sdp: String::new() }) } }) }
        fn set_local_description(&mut self, desc: &SessionDescription) -> MediaResult<()> { self.check_closed()?; if desc.sdp.is_empty() { return Ok(()); } rt().block_on(async { let sd = make_sd(desc)?; self.pc.set_local_description(sd).await.map_err(|e| MediaError::new(format!("{e}"))) }) }
        fn set_remote_description(&mut self, desc: &SessionDescription) -> MediaResult<()> { self.check_closed()?; if desc.sdp.is_empty() { return Ok(()); } rt().block_on(async { let sd = make_sd(desc)?; self.pc.set_remote_description(sd).await.map_err(|e| MediaError::new(format!("{e}"))) }) }
        fn add_ice_candidate(&mut self, candidate: &str, sdp_mid: &str) -> MediaResult<()> { self.check_closed()?; if candidate.is_empty() { return Ok(()); } rt().block_on(async { self.pc.add_ice_candidate(RTCIceCandidateInit { candidate: candidate.to_string(), sdp_mid: Some(sdp_mid.to_string()), sdp_mline_index: Some(0), username_fragment: None }).await.or_else(|_| Ok(())) }) }
        fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> { self.check_closed()?; rt().block_on(async { let dc = self.pc.create_data_channel(label, None).await.map_err(|e| MediaError::new(format!("{e}")))?; Ok(Box::new(NativeDataChannel { dc }) as Box<dyn DataChannel>) }) }
        fn ice_connection_state(&self) -> IceConnectionState { use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState as W; match self.pc.ice_connection_state() { W::New => IceConnectionState::New, W::Checking => IceConnectionState::Checking, W::Connected => IceConnectionState::Connected, W::Completed => IceConnectionState::Completed, W::Failed => IceConnectionState::Failed, W::Disconnected => IceConnectionState::Disconnected, W::Closed => IceConnectionState::Closed, _ => IceConnectionState::New } }
        fn connection_state(&self) -> ConnectionState { use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState as W; match self.pc.connection_state() { W::New => ConnectionState::New, W::Connecting => ConnectionState::Connecting, W::Connected => ConnectionState::Connected, W::Disconnected => ConnectionState::Disconnected, W::Failed => ConnectionState::Failed, W::Closed => ConnectionState::Closed, _ => ConnectionState::New } }
        fn gathering_state(&self) -> GatheringState { use webrtc::ice_transport::ice_gathering_state::RTCIceGatheringState as W; match self.pc.ice_gathering_state() { W::New => GatheringState::New, W::Gathering => GatheringState::Gathering, W::Complete => GatheringState::Complete, _ => GatheringState::New } }
        fn signaling_state(&self) -> SignalingState { use webrtc::peer_connection::signaling_state::RTCSignalingState as W; match self.pc.signaling_state() { W::Stable => SignalingState::Stable, W::HaveLocalOffer => SignalingState::HaveLocalOffer, W::HaveLocalPranswer => SignalingState::HaveLocalPranswer, W::HaveRemoteOffer => SignalingState::HaveRemoteOffer, W::HaveRemotePranswer => SignalingState::HaveRemotePranswer, _ => SignalingState::Stable } }
        fn local_description(&self) -> MediaResult<SessionDescription> { rt().block_on(async { self.pc.local_description().await.map(|d| SessionDescription { sdp_type: "local".into(), sdp: d.sdp }).ok_or_else(|| MediaError::new("no local desc")) }) }
        fn remote_description(&self) -> MediaResult<SessionDescription> { rt().block_on(async { self.pc.remote_description().await.map(|d| SessionDescription { sdp_type: "remote".into(), sdp: d.sdp }).ok_or_else(|| MediaError::new("no remote desc")) }) }
        fn close(&mut self) -> MediaResult<()> { rt().block_on(async { self.pc.close().await.map_err(|e| MediaError::new(format!("{e}"))) }) }

        fn create_video_track(&self, source: Box<dyn crate::video::source_sink::VideoSource<crate::video::frame::BoxVideoFrame>>) -> MediaResult<Box<dyn VideoTrack>> {
            use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
            use crate::video::source_sink::VideoSinkWants;
            use std::sync::Mutex as StdMutex;

            eprintln!("[gkit] create_video_track: creating track (VP8)");

            let tls = Arc::new(TrackLocalStaticSample::new(
                webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability { mime_type: webrtc::api::media_engine::MIME_TYPE_VP8.to_string(), ..Default::default() },
                "video0".into(), "gkit".into(),
            ));

            let tls_w = tls.clone();
            let frame_count = Arc::new(std::sync::atomic::AtomicU64::new(0)); let fc = frame_count.clone();
            source.add_or_update_sink(Box::new(EncoderSink { tls: tls_w, count: fc }), VideoSinkWants { is_active: true, ..Default::default() });

            struct EncoderSink { tls: Arc<webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample>, count: Arc<std::sync::atomic::AtomicU64> }
            impl crate::video::source_sink::VideoSink<crate::video::frame::BoxVideoFrame> for EncoderSink {
                fn on_frame(&self, frame: &crate::video::frame::BoxVideoFrame) {
                    let n = self.count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if n % 30 == 1 { eprintln!("[gkit] Sink frame #{}", n); }
                    if let Ok(i420) = frame.buffer.to_i420() {
                        let raw: Vec<u8> = i420.data_y.iter().chain(&i420.data_u).chain(&i420.data_v).copied().collect();
                        let _ = rt().block_on(self.tls.write_sample(&webrtc::media::Sample { data: bytes::Bytes::from(raw), duration: std::time::Duration::from_micros(66_666), timestamp: std::time::SystemTime::now(), ..Default::default() }));
                    }
                }
            }
            struct WrtcVideoTrack { id: String, sinks: std::sync::Mutex<Vec<Box<dyn crate::video::source_sink::VideoSink<crate::video::frame::BoxVideoFrame>>>>, _source: StdMutex<Option<Box<dyn crate::video::source_sink::VideoSource<crate::video::frame::BoxVideoFrame>>>>, _tls: Option<Arc<webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample>> }
            impl VideoTrack for WrtcVideoTrack {
                fn id(&self) -> &str { &self.id } fn kind(&self) -> &str { "video" }
                fn add_sink(&self, sink: Box<dyn crate::video::source_sink::VideoSink<crate::video::frame::BoxVideoFrame>>) { self.sinks.lock().unwrap().push(sink); }
            }
            let pc = self.pc.clone(); let tls_for_add = tls.clone();
            rt().block_on(async move { pc.add_track(tls_for_add).await.map(|_| ()).map_err(|e| MediaError::new(format!("{e}"))) })?;
            Ok(Box::new(WrtcVideoTrack { id: "video0".into(), sinks: std::sync::Mutex::new(Vec::new()), _source: StdMutex::new(Some(source)), _tls: Some(tls) }))
        }

        fn set_on_track(&self, cb: Box<dyn Fn(Box<dyn VideoTrack>) + Send>) {
            use crate::video::buffer::I420Buffer;
            use openh264::formats::YUVSource;
            let cb = Arc::new(std::sync::Mutex::new(Some(cb)));
            self.pc.on_track(Box::new(move |track, _receiver, _transceiver| {
                eprintln!("[gkit] on_track fired! track={}", track.id());
                let decoder = Arc::new(std::sync::Mutex::new(openh264::decoder::Decoder::new().unwrap()));
                let sinks: Arc<std::sync::Mutex<Vec<Box<dyn crate::video::source_sink::VideoSink<crate::video::frame::BoxVideoFrame>>>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
                let rmt_sinks = sinks.clone();
                let gkit_track: Box<dyn VideoTrack> = Box::new(RmtVideoTrack { id: track.id().to_string(), sinks: rmt_sinks });
                if let Ok(lock) = cb.lock() { if let Some(ref f) = *lock { f(gkit_track); } }
                let dec = decoder.clone(); let dec_sinks = sinks.clone();
                tokio::spawn(async move {
                    let mut pkt_count = 0u64;
                    loop { match track.read_rtp().await { Ok((pkt, _)) => { pkt_count += 1; if pkt_count % 30 == 1 { eprintln!("[gkit] decoder RTP pkt #{}", pkt_count); } let mut d = dec.lock().unwrap(); if let Ok(Some(yuv)) = d.decode(&pkt.payload) { let w = yuv.dimensions().0 as u32; let h = yuv.dimensions().1 as u32; let mut i420 = I420Buffer::new(w, h); i420.data_y.copy_from_slice(yuv.y()); i420.data_u.copy_from_slice(yuv.u()); i420.data_v.copy_from_slice(yuv.v()); let frame = crate::video::frame::BoxVideoFrame::new(Box::new(i420)); for sink in dec_sinks.lock().unwrap().iter() { sink.on_frame(&frame); } } } Err(e) => { eprintln!("[gkit] decoder read_rtp err: {e}"); break; } } }
                    eprintln!("[gkit] decoder loop ended after {} pkts", pkt_count);
                });
                Box::pin(async {})
            }));
        }

        fn set_on_ice_candidate(&self, cb: Box<dyn Fn(IceCandidate) + Send>) { let cb = Arc::new(std::sync::Mutex::new(Some(cb))); let c = cb.clone(); self.pc.on_ice_candidate(Box::new(move |cand: Option<webrtc::ice_transport::ice_candidate::RTCIceCandidate>| { if let Some(cand) = cand { if let Ok(lock) = c.lock() { if let Some(ref f) = *lock { f(IceCandidate { candidate: cand.to_json().unwrap().candidate, sdp_mid: None, sdp_mline_index: None }); } } } Box::pin(async {}) })); }
        fn set_on_ice_connection_state_change(&self, cb: Box<dyn Fn(IceConnectionState) + Send>) { let cb = Arc::new(std::sync::Mutex::new(Some(cb))); self.pc.on_ice_connection_state_change(Box::new(move |s: webrtc::ice_transport::ice_connection_state::RTCIceConnectionState| { let mapped = match s { webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::New => IceConnectionState::New, webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Checking => IceConnectionState::Checking, webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Connected => IceConnectionState::Connected, webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Completed => IceConnectionState::Completed, webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Failed => IceConnectionState::Failed, webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Disconnected => IceConnectionState::Disconnected, webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::Closed => IceConnectionState::Closed, _ => IceConnectionState::New }; if let Ok(lock) = cb.lock() { if let Some(ref f) = *lock { f(mapped); } } Box::pin(async {}) })); }
        fn gather_complete(&self) -> MediaResult<()> { rt().block_on(async { let mut rx = self.pc.gathering_complete_promise().await; let _ = rx.recv().await; Ok(()) }) }
    }

    impl DataChannel for NativeDataChannel {
        fn label(&self) -> &str { "" }
        fn ready_state(&self) -> DataChannelState { use webrtc::data_channel::data_channel_state::RTCDataChannelState as W; match self.dc.ready_state() { W::Open => DataChannelState::Open, W::Closed => DataChannelState::Closed, _ => DataChannelState::Connecting } }
        fn send_text(&self, data: &str) -> MediaResult<()> { rt().block_on(async { self.dc.send_text(data).await.map(|_| ()).map_err(|e| MediaError::new(format!("{e}"))) }) }
        fn send_bytes(&self, data: &[u8]) -> MediaResult<()> { rt().block_on(async { self.dc.send(&bytes::Bytes::copy_from_slice(data)).await.map(|_| ()).map_err(|e| MediaError::new(format!("{e}"))) }) }
        fn close(&mut self) -> MediaResult<()> { rt().block_on(async { self.dc.close().await.map_err(|e| MediaError::new(format!("{e}"))) }) }
    }

    impl PeerConnectionFactory for NativeFactory {
        type PC = NativePeerConnection;
        fn create_peer_connection(&self) -> MediaResult<Self::PC> { NativePeerConnection::new() }
        fn create_peer_connection_with_config(&self, _c: &RtcConfiguration) -> MediaResult<Self::PC> { NativePeerConnection::new() }
    }

    impl NativeDataChannel {
        pub fn new(_label: &str) -> Self { panic!("NativeDataChannel must be created via PeerConnection::create_data_channel") }
    }

    struct RmtVideoTrack { id: String, sinks: Arc<std::sync::Mutex<Vec<Box<dyn crate::video::source_sink::VideoSink<crate::video::frame::BoxVideoFrame>>>>> }
    impl VideoTrack for RmtVideoTrack {
        fn id(&self) -> &str { &self.id } fn kind(&self) -> &str { "video" }
        fn add_sink(&self, sink: Box<dyn crate::video::source_sink::VideoSink<crate::video::frame::BoxVideoFrame>>) { self.sinks.lock().unwrap().push(sink); }
    }
}

// ============================================================================
// Stub implementation (when webrtc-rs feature is NOT enabled)
// ============================================================================

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

// ============================================================================
// Public exports
// ============================================================================

#[cfg(feature = "backend-native-webrtc-rs")]
pub use real::*;
#[cfg(not(feature = "backend-native-webrtc-rs"))]
pub use stub::*;
