use gkit_media::protocols::rtc::client::core::VideoTrack;
use gkit_media::video::frame::BoxVideoFrame;
use gkit_media::video::source_sink::VideoSink;
use libwebrtc::video_track::RtcVideoTrack as LkRtcVideoTrack;

/// Thin adapter wrapping a libwebrtc `RtcVideoTrack` to implement gkit's `VideoTrack` trait.
pub struct LkVideoTrack {
    pub(crate) inner: LkRtcVideoTrack,
    id: String,
}

impl LkVideoTrack {
    pub fn new(inner: LkRtcVideoTrack) -> Self {
        let id = inner.id();
        Self { inner, id }
    }
}

impl VideoTrack for LkVideoTrack {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "video"
    }

    fn add_sink(&self, _sink: Box<dyn VideoSink<BoxVideoFrame>>) {
        // TODO: For receiver-side tracks (from on_track callback), route decoded
        // frames to the registered sinks. The libwebrtc RtcVideoTrack does not
        // directly expose a per-sink callback on the Rust side yet. Once the
        // native video_stream provides frame delivery, wire it here.
    }
}
