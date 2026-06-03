use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use libwebrtc::data_channel::DataChannelInit;
use libwebrtc::media_stream_track::MediaStreamTrack;
use libwebrtc::peer_connection::AnswerOptions;
use libwebrtc::peer_connection::OfferOptions;
use libwebrtc::peer_connection_factory::native::PeerConnectionFactoryExt;
use libwebrtc::rtp_receiver::RtpReceiver as LkRtpReceiver;
use libwebrtc::rtp_sender::RtpSender as LkRtpSender;
use libwebrtc::rtp_transceiver::RtpTransceiver as LkRtpTransceiver;
use libwebrtc::stats::RtcStats;
use libwebrtc::video_source::VideoResolution;
use libwebrtc::video_source::native::NativeVideoSource;

use gkit_media::protocols::rtc::peer::{
    ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState, MediaError,
    MediaResult, PeerConnection, SessionDescription, SignalingState, VideoTrack,
};
use gkit_media::video::frame::BoxVideoFrame;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};

use crate::adapt::data_channel::LkDataChannelAdapter;
use crate::adapt::factory::get_pcf;
use crate::adapt::ice::lk_ice_from_parts;
use crate::adapt::session_description::lk_sdp_from_core;
use crate::adapt::video_frame::gkit_box_frame_to_lk;
use crate::adapt::video_track::LkVideoTrack;

pub(crate) fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<&'static tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        let rt = Box::leak(Box::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("tokio runtime"),
        ));
        livekit_runtime::set_handle(rt.handle().clone());
        let handle = rt.handle().clone();
        std::thread::Builder::new()
            .name("gkit-webrtc-rt".into())
            .spawn(move || {
                handle.block_on(std::future::pending::<()>());
            })
            .expect("spawn tokio runtime thread");
        rt
    })
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
        rt().block_on(async {
            self.inner
                .create_offer(OfferOptions::default())
                .await
                .map(crate::adapt::convert::lk_sdp_to_core)
                .map_err(|e| MediaError::new(format!("create_offer: {e}")))
        })
    }

    fn create_answer(&self) -> MediaResult<SessionDescription> {
        rt().block_on(async {
            self.inner
                .create_answer(AnswerOptions::default())
                .await
                .map(crate::adapt::convert::lk_sdp_to_core)
                .map_err(|e| MediaError::new(format!("create_answer: {e}")))
        })
    }

    fn set_local_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        let lk_sdp = lk_sdp_from_core(desc)
            .map_err(|e| MediaError::new(format!("set_local_description: {e}")))?;
        rt().block_on(async {
            self.inner
                .set_local_description(lk_sdp)
                .await
                .map_err(|e| MediaError::new(format!("set_local_description: {e}")))
        })
    }

    fn set_remote_description(&mut self, desc: &SessionDescription) -> MediaResult<()> {
        let lk_sdp = lk_sdp_from_core(desc)
            .map_err(|e| MediaError::new(format!("set_remote_description: {e}")))?;
        rt().block_on(async {
            self.inner
                .set_remote_description(lk_sdp)
                .await
                .map_err(|e| MediaError::new(format!("set_remote_description: {e}")))
        })
    }

    fn add_ice_candidate(&mut self, candidate: &str, sdp_mid: &str) -> MediaResult<()> {
        let lk_ice = lk_ice_from_parts(candidate, sdp_mid)
            .map_err(|e| MediaError::new(format!("add_ice_candidate: {e}")))?;
        rt().block_on(async {
            self.inner
                .add_ice_candidate(lk_ice)
                .await
                .map_err(|e| MediaError::new(format!("add_ice_candidate: {e}")))
        })
    }

    fn create_data_channel(&self, label: &str) -> MediaResult<Box<dyn DataChannel>> {
        self.inner
            .create_data_channel(label, DataChannelInit::default())
            .map(|dc| Box::new(LkDataChannelAdapter::new(dc)) as Box<dyn DataChannel>)
            .map_err(|e| MediaError::new(format!("create_data_channel: {e}")))
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        crate::adapt::convert::lk_ice_state(self.inner.ice_connection_state())
    }

    fn connection_state(&self) -> ConnectionState {
        crate::adapt::convert::lk_conn_state(self.inner.connection_state())
    }

    fn gathering_state(&self) -> GatheringState {
        crate::adapt::convert::lk_gathering_state(self.inner.ice_gathering_state())
    }

    fn signaling_state(&self) -> SignalingState {
        crate::adapt::convert::lk_signaling_state(self.inner.signaling_state())
    }

    fn local_description(&self) -> MediaResult<SessionDescription> {
        self.inner
            .current_local_description()
            .map(crate::adapt::convert::lk_sdp_to_core)
            .ok_or_else(|| MediaError::new("no local description"))
    }

    fn remote_description(&self) -> MediaResult<SessionDescription> {
        self.inner
            .current_remote_description()
            .map(crate::adapt::convert::lk_sdp_to_core)
            .ok_or_else(|| MediaError::new("no remote description"))
    }

    fn set_on_ice_candidate(
        &self,
        cb: Box<dyn Fn(gkit_media::protocols::rtc::peer::IceCandidate) + Send>,
    ) {
        let cb: Mutex<
            Option<Box<dyn Fn(gkit_media::protocols::rtc::peer::IceCandidate) + Send>>,
        > = Mutex::new(Some(cb));
        self.inner.on_ice_candidate(Some(Box::new(move |lk_ic| {
            if let Ok(guard) = cb.lock() {
                if let Some(ref cb) = *guard {
                    cb(crate::adapt::convert::lk_ice_candidate_to_core(lk_ic));
                }
            }
        })));
    }

    fn set_on_ice_connection_state_change(&self, cb: Box<dyn Fn(IceConnectionState) + Send>) {
        let cb: Mutex<Option<Box<dyn Fn(IceConnectionState) + Send>>> = Mutex::new(Some(cb));
        self.inner
            .on_ice_connection_state_change(Some(Box::new(move |lk_state| {
                if let Ok(guard) = cb.lock() {
                    if let Some(ref cb) = *guard {
                        cb(crate::adapt::convert::lk_ice_state(lk_state));
                    }
                }
            })));
    }

    fn close(&mut self) -> MediaResult<()> {
        self.inner.close();
        Ok(())
    }

    fn get_stats_json(&self) -> MediaResult<String> {
        rt().block_on(async { crate::adapt::stats::get_stats_json(&self.inner).await })
    }

    fn create_video_track(
        &self,
        source: Box<dyn VideoSource<BoxVideoFrame>>,
    ) -> MediaResult<Box<dyn VideoTrack>> {
        let native_source = NativeVideoSource::new(
            VideoResolution {
                width: 640,
                height: 360,
            },
            false,
        );

        Box::leak(Box::new(SourceToSinkAdapter::new(native_source.clone(), source)));
        let lk_track = get_pcf().create_video_track("video", native_source);

        let media_track: libwebrtc::media_stream_track::MediaStreamTrack =
            lk_track.clone().into();
        self.inner
            .add_track(media_track, &["stream"])
            .map_err(|e| MediaError::new(format!("add_track: {e}")))?;

        Ok(Box::new(LkVideoTrack::new(lk_track)))
    }

    fn set_on_track(&self, cb: Box<dyn Fn(Box<dyn VideoTrack>) + Send>) {
        let cb: Mutex<Option<Box<dyn Fn(Box<dyn VideoTrack>) + Send>>> = Mutex::new(Some(cb));
        let on_track: libwebrtc::peer_connection::OnTrack =
            Box::new(move |event: libwebrtc::peer_connection::TrackEvent| {
                if let Ok(mut guard) = cb.lock() {
                    if let Some(ref cb) = *guard {
                        if let MediaStreamTrack::Video(lk_vt) = event.track {
                            cb(Box::new(LkVideoTrack::new(lk_vt)));
                        }
                    }
                }
            });
        self.inner.on_track(Some(on_track));
    }
}

impl LiveKitPeerConnection {
    pub fn add_track<T: AsRef<str>>(
        &self,
        track: MediaStreamTrack,
        stream_ids: &[T],
    ) -> Result<LkRtpSender, libwebrtc::RtcError> {
        self.inner.add_track(track, stream_ids)
    }

    pub fn remove_track(&self, sender: LkRtpSender) -> Result<(), libwebrtc::RtcError> {
        self.inner.remove_track(sender)
    }

    /// Collect all WebRTC stats as a JSON string.
    pub async fn get_stats(&self) -> MediaResult<String> {
        let stats: Vec<RtcStats> = self
            .inner
            .get_stats()
            .await
            .map_err(|e| MediaError::new(format!("get_stats: {e}")))?;
        Ok(format!("{:#?}", stats))
    }

    /// Returns all RTP senders attached to this peer connection.
    pub fn senders(&self) -> Vec<crate::adapt::rtp::RtpSenderHandle> {
        self.inner
            .senders()
            .into_iter()
            .map(crate::adapt::rtp::RtpSenderHandle::new)
            .collect()
    }

    /// Returns all RTP receivers attached to this peer connection.
    pub fn receivers(&self) -> Vec<crate::adapt::rtp::RtpReceiverHandle> {
        self.inner
            .receivers()
            .into_iter()
            .map(crate::adapt::rtp::RtpReceiverHandle::new)
            .collect()
    }

    /// Returns all RTP transceivers attached to this peer connection.
    pub fn transceivers(&self) -> Vec<crate::adapt::rtp::RtpTransceiverHandle> {
        self.inner
            .transceivers()
            .into_iter()
            .map(crate::adapt::rtp::RtpTransceiverHandle::new)
            .collect()
    }
}

struct FrameForwarder {
    native_source: NativeVideoSource,
    running: Arc<AtomicBool>,
}

impl VideoSink<BoxVideoFrame> for FrameForwarder {
    fn on_frame(&self, frame: &BoxVideoFrame) {
        if self.running.load(Ordering::Relaxed) {
            let lk_frame = gkit_box_frame_to_lk(frame);
            self.native_source.capture_frame(&lk_frame);
            static SENDER_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = SENDER_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n <= 3 || n % 30 == 0 {
                let _ = std::fs::write("/tmp/gkit_sender_count.log", format!("{}", n));
            }
        }
    }
}

struct SourceToSinkAdapter {
    _source: Box<dyn VideoSource<BoxVideoFrame>>,
    running: Arc<AtomicBool>,
}

impl SourceToSinkAdapter {
    fn new(native_source: NativeVideoSource, source: Box<dyn VideoSource<BoxVideoFrame>>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let forwarder = FrameForwarder {
            native_source,
            running: Arc::clone(&running),
        };
        source.add_or_update_sink(
            Box::new(forwarder),
            VideoSinkWants {
                is_active: true,
                ..Default::default()
            },
        );
        Self {
            _source: source,
            running,
        }
    }
}

impl Drop for SourceToSinkAdapter {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use crate::adapt::*;

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
        assert_eq!(
            IceConnectionState::from(LkState::New),
            IceConnectionState::New
        );
        assert_eq!(
            IceConnectionState::from(LkState::Checking),
            IceConnectionState::Checking
        );
        assert_eq!(
            IceConnectionState::from(LkState::Connected),
            IceConnectionState::Connected
        );
        assert_eq!(
            IceConnectionState::from(LkState::Completed),
            IceConnectionState::Completed
        );
        assert_eq!(
            IceConnectionState::from(LkState::Failed),
            IceConnectionState::Failed
        );
        assert_eq!(
            IceConnectionState::from(LkState::Disconnected),
            IceConnectionState::Disconnected
        );
        assert_eq!(
            IceConnectionState::from(LkState::Closed),
            IceConnectionState::Closed
        );
        assert_eq!(
            IceConnectionState::from(LkState::Max),
            IceConnectionState::Closed
        );
    }

    #[test]
    fn gathering_state_mapping() {
        use libwebrtc::peer_connection::IceGatheringState as LkState;
        assert_eq!(GatheringState::from(LkState::New), GatheringState::New);
        assert_eq!(
            GatheringState::from(LkState::Gathering),
            GatheringState::Gathering
        );
        assert_eq!(
            GatheringState::from(LkState::Complete),
            GatheringState::Complete
        );
    }

    #[test]
    fn signaling_state_mapping() {
        use libwebrtc::peer_connection::SignalingState as LkState;
        assert_eq!(
            SignalingState::from(LkState::Stable),
            SignalingState::Stable
        );
        assert_eq!(
            SignalingState::from(LkState::HaveLocalOffer),
            SignalingState::HaveLocalOffer
        );
        assert_eq!(
            SignalingState::from(LkState::HaveLocalPrAnswer),
            SignalingState::HaveLocalPranswer
        );
        assert_eq!(
            SignalingState::from(LkState::HaveRemoteOffer),
            SignalingState::HaveRemoteOffer
        );
        assert_eq!(
            SignalingState::from(LkState::HaveRemotePrAnswer),
            SignalingState::HaveRemotePranswer
        );
        assert_eq!(
            SignalingState::from(LkState::Closed),
            SignalingState::Stable
        );
    }
}
