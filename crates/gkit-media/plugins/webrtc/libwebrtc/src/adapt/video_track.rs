use std::io::Write;
use std::sync::Mutex;

use futures::StreamExt;
use libwebrtc::video_stream::native::NativeVideoStream;
use libwebrtc::video_stream::native::NativeVideoStreamOptions;
use libwebrtc::video_track::RtcVideoTrack as LkRtcVideoTrack;

use gkit_media::protocols::rtc::client::core::VideoTrack;
use gkit_media::video::frame::BoxVideoFrame;
use gkit_media::video::source_sink::VideoSink;

/// Thin adapter wrapping a libwebrtc `RtcVideoTrack` to implement gkit's `VideoTrack` trait.
///
/// For sender-side tracks (created via `create_video_track`), frames come from
/// a `VideoSource` and are forwarded to libwebrtc's encoder. `add_sink` is
/// typically not used on sender tracks.
///
/// For receiver-side tracks (delivered via `set_on_track`), `add_sink` spawns
/// a tokio task that pulls decoded frames from `NativeVideoStream` and forwards
/// them to the registered `VideoSink`.
pub struct LkVideoTrack {
    pub(crate) inner: LkRtcVideoTrack,
    id: String,
    sinks: Mutex<Vec<std::thread::JoinHandle<()>>>,
}

impl LkVideoTrack {
    pub fn new(inner: LkRtcVideoTrack) -> Self {
        let id = inner.id();
        Self {
            inner,
            id,
            sinks: Mutex::new(Vec::new()),
        }
    }
}

impl VideoTrack for LkVideoTrack {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "video"
    }

    fn add_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>) {
        let track = self.inner.clone();
        let rt_handle = crate::adapt::peer_connection::rt().handle().clone();

        let handle = std::thread::spawn(move || {
            rt_handle.block_on(async {
                let _ = std::fs::write("/tmp/gkit_rt_start.log", "1");
                let mut stream = NativeVideoStream::with_options(track,
                    NativeVideoStreamOptions { queue_size_frames: Some(0) });
                let mut frame_count = 0u64;
                while let Some(frame) = stream.next().await {
                    frame_count += 1;
                    let _ = std::fs::write("/tmp/gkit_receiver_count.log", format!("{}", frame_count));
                    let gkit_frame = crate::adapt::video_frame::gkit_box_frame_from_lk(&frame);
                    sink.on_frame(&gkit_frame);
                    if frame_count == 1 {
                        let _ = std::fs::write("/tmp/gkit_rt_frame1.log", "1");
                    }
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
