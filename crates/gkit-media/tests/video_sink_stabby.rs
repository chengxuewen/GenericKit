use std::sync::Mutex;

use gkit_media::video::frame_stabby::{I420Planes, StableVideoFrame, VideoFrameMeta, BufferData};
use gkit_media::video_sink_stabby::IStableVideoSink;

struct CountingSink {
    count: Mutex<u32>,
}

impl IStableVideoSink for CountingSink {
    extern "C" fn on_frame_owned(&self, _frame: stabby::boxed::Box<StableVideoFrame>) {
        *self.count.lock().unwrap() += 1;
    }
    extern "C" fn on_frame(&self, _frame: &StableVideoFrame) {
        // no-op for counting
    }
    extern "C" fn on_discarded_frame(&self, _timestamp_us: i64) {
        // no-op for counting
    }
}

fn make_test_i420_frame(width: u32, height: u32) -> StableVideoFrame {
    StableVideoFrame {
        meta: VideoFrameMeta::new(width, height),
        buffer: BufferData::I420(I420Planes::zeroed(width, height)),
    }
}

#[test]
fn sink_counts_frames() {
    let sink = CountingSink {
        count: Mutex::new(0),
    };
    let frame = make_test_i420_frame(640, 480);
    sink.on_frame_owned(stabby::boxed::Box::new(frame));
    assert_eq!(*sink.count.lock().unwrap(), 1);
}

#[test]
fn multiple_frame_receives_increment_correctly() {
    let sink = CountingSink {
        count: Mutex::new(0),
    };
    for _ in 0..5 {
        sink.on_frame_owned(stabby::boxed::Box::new(make_test_i420_frame(320, 240)));
    }
    assert_eq!(*sink.count.lock().unwrap(), 5);
}

#[test]
fn on_discarded_frame_default_noop() {
    let sink = CountingSink {
        count: Mutex::new(0),
    };
    sink.on_discarded_frame(42_000);
    assert_eq!(*sink.count.lock().unwrap(), 0);
}
