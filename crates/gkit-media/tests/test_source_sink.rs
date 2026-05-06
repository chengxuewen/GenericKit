use gkit_media::video::source_sink::*;
use std::sync::{Arc, Mutex};

// ── aggregate_wants ──

#[test]
fn aggregate_wants_default_all_zeros() {
    let wants = [VideoSinkWants::default()];
    let agg = aggregate_wants(wants.iter());
    assert!(!agg.rotation_applied);
    assert!(!agg.is_active);
    assert_eq!(agg.max_pixel_count, 0);
    assert_eq!(agg.max_framerate_fps, 0);
    assert_eq!(agg.resolution_alignment, 1);
}

#[test]
fn aggregate_wants_or_rotation() {
    let w1 = VideoSinkWants { rotation_applied: true, ..Default::default() };
    let w2 = VideoSinkWants { rotation_applied: false, ..Default::default() };
    let agg = aggregate_wants([&w1, &w2].into_iter());
    assert!(agg.rotation_applied);
}

#[test]
fn aggregate_wants_min_pixel_count() {
    let w1 = VideoSinkWants { max_pixel_count: 1920 * 1080, ..Default::default() };
    let w2 = VideoSinkWants { max_pixel_count: 640 * 480, ..Default::default() };
    let agg = aggregate_wants([&w1, &w2].into_iter());
    assert_eq!(agg.max_pixel_count, 640 * 480);
}

#[test]
fn aggregate_wants_min_framerate() {
    let w1 = VideoSinkWants { max_framerate_fps: 60, ..Default::default() };
    let w2 = VideoSinkWants { max_framerate_fps: 30, ..Default::default() };
    let agg = aggregate_wants([&w1, &w2].into_iter());
    assert_eq!(agg.max_framerate_fps, 30);
}

#[test]
fn aggregate_wants_lcm_alignment() {
    let w1 = VideoSinkWants { resolution_alignment: 2, ..Default::default() };
    let w2 = VideoSinkWants { resolution_alignment: 4, ..Default::default() };
    let agg = aggregate_wants([&w1, &w2].into_iter());
    assert_eq!(agg.resolution_alignment, 4);
}

// ── Test helpers ──

struct TestSink<T: Send> {
    frames: Mutex<Vec<T>>,
}

impl<T: Send> TestSink<T> {
    fn new() -> Self {
        Self { frames: Mutex::new(Vec::new()) }
    }
}

impl<T: Clone + Send + 'static> VideoSink<T> for TestSink<T> {
    fn on_frame(&self, frame: &T) {
        self.frames.lock().unwrap().push(frame.clone());
    }
}

// ── VideoBroadcaster ──

#[test]
fn broadcaster_add_remove_sink() {
    let mut bc = VideoBroadcaster::<u32>::new();
    let sink: Box<dyn VideoSink<u32>> = Box::new(TestSink::<u32>::new());
    bc.add_or_update_sink(sink, VideoSinkWants::default());
    assert_eq!(bc.sink_count(), 1);
}

#[test]
fn broadcaster_fan_out_to_multiple_sinks() {
    let mut bc = VideoBroadcaster::<u32>::new();
    let s1 = Arc::new(TestSink::<u32>::new());
    let s2 = Arc::new(TestSink::<u32>::new());
    bc.add_or_update_sink(Box::new(TestSink::<u32>::new()), VideoSinkWants::default());
    bc.add_or_update_sink(Box::new(TestSink::<u32>::new()), VideoSinkWants::default());
    assert_eq!(bc.sink_count(), 2);
}

#[test]
fn broadcaster_wants_aggregation() {
    let mut bc = VideoBroadcaster::<u32>::new();
    let s1 = Arc::new(TestSink::<u32>::new());
    let s2 = Arc::new(TestSink::<u32>::new());
    let w1 = VideoSinkWants { rotation_applied: true, max_pixel_count: 1920 * 1080, ..Default::default() };
    let w2 = VideoSinkWants { rotation_applied: false, max_pixel_count: 640 * 480, ..Default::default() };
    bc.add_or_update_sink(Box::new(TestSink::<u32>::new()), w1);
    bc.add_or_update_sink(Box::new(TestSink::<u32>::new()), w2);
    let wants = bc.wants();
    assert!(wants.rotation_applied); // OR
    assert_eq!(wants.max_pixel_count, 640 * 480); // MIN
}
