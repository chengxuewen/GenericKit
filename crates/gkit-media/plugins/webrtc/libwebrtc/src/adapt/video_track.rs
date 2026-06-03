use std::sync::Mutex;

use futures::StreamExt;
use libwebrtc::video_stream::native::NativeVideoStream;
use libwebrtc::video_stream::native::NativeVideoStreamOptions;
use libwebrtc::video_track::RtcVideoTrack as LkRtcVideoTrack;

use gkit_media::protocols::rtc::client::core::VideoTrack;
use gkit_media::video::frame::BoxVideoFrame;
use gkit_media::video::source_sink::VideoSink;

pub struct LkVideoTrack {
    pub(crate) inner: LkRtcVideoTrack,
    id: String,
    sinks: Mutex<Vec<std::thread::JoinHandle<()>>>,
}

impl LkVideoTrack {
    pub fn new(inner: LkRtcVideoTrack) -> Self {
        let id = inner.id();
        Self { inner, id, sinks: Mutex::new(Vec::new()) }
    }
}

impl VideoTrack for LkVideoTrack {
    fn id(&self) -> &str { &self.id }
    fn kind(&self) -> &str { "video" }

    fn add_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>) {
        let track = self.inner.clone();
        let rt_handle = crate::adapt::peer_connection::rt().handle().clone();
        let handle = std::thread::spawn(move || {
            rt_handle.block_on(async {
                let mut stream = NativeVideoStream::with_options(track,
                    NativeVideoStreamOptions { queue_size_frames: Some(0) });
                let mut frame_count = 0u64;
                while let Some(frame) = stream.next().await {
                    frame_count += 1;
                    let _ = std::fs::write("/tmp/gkit_receiver_count.log", format!("{}", frame_count));
                    let gkit_frame = crate::adapt::video_frame::gkit_box_frame_from_lk(&frame);
                    sink.on_frame(&gkit_frame);
                }
                let _ = std::fs::write("/tmp/gkit_rt_end.log", format!("STREAM_END:{}", frame_count));
            });
        });
        self.sinks.lock().unwrap().push(handle);
    }
}

impl Drop for LkVideoTrack {
    fn drop(&mut self) {
        self.sinks.lock().unwrap().clear();
    }
}
