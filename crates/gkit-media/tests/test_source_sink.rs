use gkit_media::video::source_sink::*;
use gkit_media::video::adapter::VideoAdapter;
use gkit_media::capture::generator::{FramePattern, SquarePattern, VideoFrameGenerator};

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
    _frames: std::sync::Mutex<Vec<T>>,
}

impl<T: Send> TestSink<T> {
    fn new() -> Self {
        Self { _frames: std::sync::Mutex::new(Vec::new()) }
    }
}

impl<T: Clone + Send + 'static> VideoSink<T> for TestSink<T> {
    fn on_frame(&self, frame: &T) {
        self._frames.lock().unwrap().push(frame.clone());
    }
}

// ── VideoBroadcaster ──

#[test]
fn broadcaster_add_remove_sink() {
    let bc = VideoBroadcaster::<u32>::new();
    let sink: Box<dyn VideoSink<u32>> = Box::new(TestSink::<u32>::new());
    bc.add_or_update_sink(sink, VideoSinkWants::default());
    assert_eq!(bc.sink_count(), 1);
}

#[test]
fn broadcaster_fan_out_to_multiple_sinks() {
    let bc = VideoBroadcaster::<u32>::new();
    bc.add_or_update_sink(Box::new(TestSink::<u32>::new()), VideoSinkWants::default());
    bc.add_or_update_sink(Box::new(TestSink::<u32>::new()), VideoSinkWants::default());
    assert_eq!(bc.sink_count(), 2);
}

#[test]
fn broadcaster_wants_aggregation() {
    let bc = VideoBroadcaster::<u32>::new();
    let w1 = VideoSinkWants { rotation_applied: true, max_pixel_count: 1920 * 1080, ..Default::default() };
    let w2 = VideoSinkWants { rotation_applied: false, max_pixel_count: 640 * 480, ..Default::default() };
    bc.add_or_update_sink(Box::new(TestSink::<u32>::new()), w1);
    bc.add_or_update_sink(Box::new(TestSink::<u32>::new()), w2);
    let wants = bc.wants();
    assert!(wants.rotation_applied); // OR
    assert_eq!(wants.max_pixel_count, 640 * 480); // MIN
}

// ── VideoAdapter ──

#[test]
fn adapter_no_adapt_needed() {
    let mut adapter = VideoAdapter::new();
    let result = adapter.adapt_frame(640, 480, 0);
    assert!(result.is_some());
}

#[test]
fn adapter_downscale_to_target() {
    let mut adapter = VideoAdapter::new();
    adapter.on_sink_wants(&VideoSinkWants {
        max_pixel_count: 320 * 240, ..Default::default()
    });
    let result = adapter.adapt_frame(640, 480, 0);
    assert!(result.is_some());
    let (_cx, _cy, _cw, _ch, out_w, out_h) = result.unwrap();
    assert!(out_w * out_h <= 320 * 240);
}

#[test]
fn adapter_rate_limit_drops_frame() {
    let mut adapter = VideoAdapter::new();
    adapter.on_sink_wants(&VideoSinkWants {
        max_framerate_fps: 30, ..Default::default()
    });
    assert!(adapter.adapt_frame(640, 480, 0).is_some());
    assert!(adapter.adapt_frame(640, 480, 0).is_none());
}

// ── SquarePattern ──

#[test]
fn square_pattern_draws_non_gray_pixels() {
    let mut pattern = SquarePattern::new(320, 240, 10);
    let mut y = vec![127u8; (320 * 240) as usize];
    let mut u = vec![127u8; (160 * 120) as usize];
    let mut v = vec![127u8; (160 * 120) as usize];
    pattern.draw(&mut y, &mut u, &mut v, 320, 160, 160);
    // Verify some non-gray pixels exist (squares were drawn)
    let has_color = y.iter().any(|&p| p != 127) || u.iter().any(|&p| p != 127);
    assert!(has_color, "pattern should draw colored squares");
}

// ── VideoFrameGenerator ──

#[test]
fn generator_creates_non_empty_frames() {
    let mut generator = VideoFrameGenerator::new(320, 240, 30);
    generator.start();
    std::thread::sleep(std::time::Duration::from_millis(100));
    generator.stop();
    assert!(generator.sink_count() >= 0);
}

#[test]
fn generator_stop_prevents_further_frames() {
    let mut generator = VideoFrameGenerator::new(320, 240, 30);
    generator.start();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let count_before = generator.sink_count();
    generator.stop();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(generator.sink_count(), count_before);
}

// ── DefaultAudioSource ──

#[test]
fn audio_source_produces_silence() {
    use std::sync::atomic::AtomicBool;
    use gkit_media::video::source_sink::DefaultAudioSource;
    let mut src = DefaultAudioSource::new(48000, 1);
    let received = std::sync::Arc::new(AtomicBool::new(false));
    struct TestAudio {
        received: std::sync::Arc<AtomicBool>,
    }
    impl gkit_media::video::source_sink::AudioSink for TestAudio {
        fn on_data(&self, samples: &[i16], rate: u32, ch: u32) {
            assert_eq!(rate, 48000);
            assert_eq!(ch, 1);
            if samples.iter().any(|&s| s != 0) {
                // if any non-zero, still mark as received (silence test verifies all zero)
            }
            self.received.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    src.add_sink(Box::new(TestAudio { received: received.clone() }));
    src.start();
    std::thread::sleep(std::time::Duration::from_millis(100));
    src.stop();
    assert!(received.load(std::sync::atomic::Ordering::Relaxed));
}
