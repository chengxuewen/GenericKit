# Video Source/Sink & VideoFrameGenerator Design

**Date**: 2026-05-06
**Priority**: P0 (highest precedence over other WebRTC phases)
**Scope**: Define pipeline traits (VideoSource, VideoSink, AudioSource, AudioSink), implement VideoBroadcaster, VideoAdapter, AdaptedVideoSource, VideoFrameGenerator (SquarePattern), DefaultAudioSource, C/C++ FFI bindings, and comprehensive unit tests
**Constraint**: All code in single crate `gkit-media`; no new workspace members

---

## 1. Design Principles

Derived from three reference implementations (libwebrtc `api/video/`, OpenCTK `libs/media/source/`, webrtc-rs):

| Pattern | Source | Reason |
|---------|--------|--------|
| Generic trait `Source<F>` / `Sink<F>` | libwebrtc `VideoSourceInterface<T>` | Single trait family works for raw `VideoFrame` and `RecordableEncodedFrame` |
| `add_or_update_sink` (single method) | libwebrtc + OpenCTK | Sinks re-express wants dynamically without remove+re-add |
| Broadcaster IS-A Sink AND Source | libwebrtc `VideoBroadcaster` | Graceful relay: receives upstream frame → fans out to downstream sinks |
| `VideoSinkWants` aggregation (MIN/MAX/LCM) | libwebrtc | Proven algorithm for multi-sink constraint reconciliation |
| `VideoAdapter` as standalone utility | libwebrtc + OpenCTK | Single-responsibility: crop → scale → rate-limit |
| `AdaptedVideoSource` decorator | libwebrtc `AdaptedVideoTrackSource` | Composes adapter + broadcaster around inner source |
| Audio: no broadcaster, no adapter | Both | Audio pipeline is simpler; raw PCM pass-through suffices |

---

## 2. Architecture

```
crates/gkit-media/src/
├── webrtc/
│   └── client/
│       ├── core.rs                    # (existing) PeerConnection, DataChannel traits
│       ├── source_sink.rs             # [NEW] VideoSource<F>, VideoSink<F>, AudioSource, AudioSink,
│       │                              #       VideoBroadcaster, VideoSinkWants
│       ├── adapter.rs                 # [NEW] VideoAdapter, AdaptedVideoSource
│       └── ...
├── video/
│   ├── mod.rs                         # + pub mod generator;
│   ├── generator.rs                   # [NEW] VideoFrameGenerator + FramePattern + SquarePattern
│   └── ...

apis/
├── c/gkit-media/src/lib.rs            # +24 new C FFI functions
├── c/gkit-media/tests/test_source_sink.c     # [NEW] 5 C Unity tests
├── cpp/gkit-media/gkit_media_source_sink.hpp # [NEW] C++ RAII wrappers
├── cpp/gkit-media/tests/test_source_sink.cpp # [NEW] 7 GTest tests
crates/gkit-media/tests/test_source_sink.rs   # [NEW] 18 Rust unit tests
```

**WebRTC Integration Point**:
```
VideoFrameGenerator → VideoBroadcaster → VideoSink (callback)
                                            │
                                            ├── C++ Viewer (display)
                                            └── webrtc-rs TrackLocalStaticSample (streaming)
```

---

## 3. Core Trait Definitions (`source_sink.rs`)

### 3.1 VideoSinkWants

```rust
/// Drives adaptation. Mirrors libwebrtc's VideoSinkWants.
#[derive(Debug, Clone)]
pub struct VideoSinkWants {
    pub rotation_applied: bool,      // sink requires pre-rotated frames
    pub max_pixel_count: u32,        // hard upper bound (0 = no limit)
    pub max_framerate_fps: u32,      // hard upper bound (0 = no limit)
    pub resolution_alignment: u32,   // e.g., 2 for I420
    pub is_active: bool,             // sink is actively encoding
}

impl Default for VideoSinkWants {
    fn default() -> Self {
        Self {
            rotation_applied: false,
            max_pixel_count: 0,
            max_framerate_fps: 0,
            resolution_alignment: 1,
            is_active: false,
        }
    }
}
```

### 3.2 Video Pipeline Traits

```rust
pub trait VideoSink<F>: Send {
    fn on_frame(&self, frame: &F);
    fn on_discarded_frame(&self) {}
}

pub trait VideoSource<F>: Send {
    fn add_or_update_sink(&mut self, sink: Box<dyn VideoSink<F>>, wants: VideoSinkWants);
    fn remove_sink(&mut self, sink: &dyn VideoSink<F>);
}
```

### 3.3 VideoBroadcaster

```rust
/// IS-A VideoSink (upstream) AND VideoSource (downstream).
/// Threadsafe fan-out with wants aggregation.
pub struct VideoBroadcaster<F> {
    pairs: Mutex<Vec<(Box<dyn VideoSink<F>>, VideoSinkWants)>>,
}

impl<F> VideoBroadcaster<F> {
    pub fn new() -> Self { Self { pairs: Mutex::new(Vec::new()) } }
    pub fn wants(&self) -> VideoSinkWants { ... }  // aggregate_wants()
    pub fn sink_count(&self) -> usize { ... }
}

impl<F: Send + 'static> VideoSink<F> for VideoBroadcaster<F> {
    fn on_frame(&self, frame: &F) {
        // fan out to all active sinks
    }
}

impl<F: Send + 'static> VideoSource<F> for VideoBroadcaster<F> {
    fn add_or_update_sink(&mut self, sink: Box<dyn VideoSink<F>>, wants: VideoSinkWants) { ... }
    fn remove_sink(&mut self, sink: &dyn VideoSink<F>) { ... }
}
```

### 3.4 Wants Aggregation Algorithm

```rust
/// MIN for constraints, OR for boolean flags, LCM for alignment
pub fn aggregate_wants<'a>(wants: impl Iterator<Item = &'a VideoSinkWants>) -> VideoSinkWants {
    // rotation_applied = OR across all
    // is_active = OR across all
    // max_pixel_count = MIN (tightest)
    // max_framerate_fps = MIN (tightest)
    // resolution_alignment = LCM
}
```

### 3.5 Audio Traits

```rust
pub trait AudioSink: Send {
    fn on_data(&self, samples: &[i16], sample_rate: u32, channels: u32);
}

pub trait AudioSource: Send {
    fn add_sink(&mut self, sink: Box<dyn AudioSink>);
    fn remove_sink(&mut self, sink: &dyn AudioSink);
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u32;
}
```

---

## 4. VideoAdapter (`adapter.rs`)

```rust
pub struct VideoAdapter {
    target_pixels: u32,
    max_fps: f32,
    frame_timestamps: VecDeque<i64>,
}

impl VideoAdapter {
    pub fn new() -> Self { ... }

    pub fn on_sink_wants(&mut self, wants: &VideoSinkWants) {
        // Apply max_pixel_count and max_framerate_fps constraints
    }

    /// Returns None to drop frame, or Some((crop_x, crop_y, crop_w, crop_h, out_w, out_h))
    pub fn adapt_frame(&mut self, in_w: u32, in_h: u32, timestamp_us: i64)
        -> Option<(u32, u32, u32, u32, u32, u32)>
    {
        // 1. Rate-limit check (window-based frame counting)
        // 2. Resolution downscale if exceeding target_pixels
        // 3. Return crop+scale params or None to drop
    }
}
```

### AdaptedVideoSource (Decorator)

```rust
pub struct AdaptedVideoSource {
    adapter: Mutex<VideoAdapter>,
    broadcaster: VideoBroadcaster<VideoFrame>,
}

impl AdaptedVideoSource {
    pub fn new() -> Self { ... }
    pub fn on_frame(&self, frame: &VideoFrame) {
        // adapter.adapt_frame() → broadcaster.on_frame() or drop
    }
}

impl VideoSource<VideoFrame> for AdaptedVideoSource {
    fn add_or_update_sink(&mut self, sink: Box<dyn VideoSink<VideoFrame>>, wants: VideoSinkWants) {
        self.adapter.lock().unwrap().on_sink_wants(&wants);
        self.broadcaster.add_or_update_sink(sink, wants);
    }
    fn remove_sink(&mut self, sink: &dyn VideoSink<VideoFrame>) {
        self.broadcaster.remove_sink(sink);
    }
}
```

---

## 5. VideoFrameGenerator (`video/generator.rs`)

### 5.1 FramePattern Trait

```rust
pub trait FramePattern: Send {
    fn draw(&mut self, y: &mut [u8], u: &mut [u8], v: &mut [u8],
            stride_y: u32, stride_u: u32, stride_v: u32);
}
```

### 5.2 SquarePattern (Default)

As in OpenCTK:
- Gray I420 background (Y=127, U=127, V=127)
- N randomly-sized, randomly-colored rectangles
- Each frame, squares move toward lower-right by small random offset (0-4 pixels)
- Timestamp overlay via embedded 6×10 bitmap font at position (10, 30), scale 2×
- No libyuv dependency — direct pixel writes to I420 planes

### 5.3 VideoFrameGenerator Struct

```rust
pub struct VideoFrameGenerator {
    broadcaster: VideoBroadcaster<VideoFrame>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl VideoFrameGenerator {
    pub fn new(width: u32, height: u32, fps: u32) -> Self { ... }
    pub fn new_with_pattern(width: u32, height: u32, fps: u32, pattern: Box<dyn FramePattern>) -> Self { ... }
    pub fn start(&mut self) { self.running.store(true, Ordering::Relaxed); }
    pub fn stop(&mut self) { self.running.store(false, Ordering::Relaxed); /* join thread */ }
    pub fn is_running(&self) -> bool { self.running.load(Ordering::Relaxed) }
}

impl VideoSource<VideoFrame> for VideoFrameGenerator {
    // delegates to self.broadcaster
}

impl Drop for VideoFrameGenerator {
    fn drop(&mut self) { self.stop(); }
}
```

**Internal thread**: sleep(1/fps) → I420Buffer::new() → pattern.draw() → VideoFrame::new() → broadcaster.on_frame()

---

## 6. DefaultAudioSource

```rust
pub struct DefaultAudioSource {
    sample_rate: u32,
    channels: u32,
    sinks: Mutex<Vec<Box<dyn AudioSink>>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl DefaultAudioSource {
    pub fn new(sample_rate: u32, channels: u32) -> Self { ... }
    pub fn start(&mut self) { ... }
    pub fn stop(&mut self) { ... }
}

impl AudioSource for DefaultAudioSource { ... }
```

Produces silence (all-zero int16 samples) at 20ms intervals.

---

## 7. C FFI (14 new functions in `apis/c/gkit-media/src/lib.rs`)

```c
// ── VideoSource (creates VideoFrameGenerator) ──
void* gkit_media_rtc_video_source_create_generator(uint32_t w, uint32_t h, uint32_t fps);
void  gkit_media_rtc_video_source_destroy(void* handle);
int   gkit_media_rtc_video_source_start(void* handle);
int   gkit_media_rtc_video_source_stop(void* handle);

// ── VideoSink (callback-based) ──
typedef void (*gkit_media_rtc_video_frame_callback_t)(void* frame_handle, void* user_data);
void* gkit_media_rtc_video_sink_create(gkit_media_rtc_video_frame_callback_t cb, void* user_data);
void  gkit_media_rtc_video_sink_destroy(void* handle);

// ── Source↔Sink wiring ──
int   gkit_media_rtc_video_source_add_sink(void* src, void* sink);
int   gkit_media_rtc_video_source_remove_sink(void* src, void* sink);

// ── AudioSource ──
void* gkit_media_rtc_audio_source_create_default(uint32_t sample_rate, uint32_t channels);
void  gkit_media_rtc_audio_source_destroy(void* handle);
int   gkit_media_rtc_audio_source_start(void* handle);
int   gkit_media_rtc_audio_source_stop(void* handle);

// ── AudioSink ──
typedef void (*gkit_media_rtc_audio_data_callback_t)(const int16_t* data, size_t frames,
    uint32_t sample_rate, uint32_t channels, void* user_data);
void* gkit_media_rtc_audio_sink_create(gkit_media_rtc_audio_data_callback_t cb, void* user_data);
void  gkit_media_rtc_audio_sink_destroy(void* handle);

// ── Audio wiring ──
int   gkit_media_rtc_audio_source_add_sink(void* src, void* sink);
int   gkit_media_rtc_audio_source_remove_sink(void* src, void* sink);
```

---

## 8. C++ Wrappers (new file `apis/cpp/gkit-media/gkit_media_source_sink.hpp`)

```cpp
namespace gkit {

class VideoSource {
public:
    static VideoSource createGenerator(uint32_t w, uint32_t h, uint32_t fps);
    ~VideoSource();
    VideoSource(VideoSource&&) noexcept;
    VideoSource& operator=(VideoSource&&) noexcept;
    void start();
    void stop();
    bool isRunning() const;
    void addSink(VideoSink& sink, const VideoSinkWants& wants = {});
    void removeSink(VideoSink& sink);
    bool valid() const;
private:
    void* handle_ = nullptr;
};

class VideoSink {
public:
    using Callback = std::function<void(VideoFrame&)>;
    explicit VideoSink(Callback callback);
    ~VideoSink();
    VideoSink(VideoSink&&) noexcept;
    VideoSink& operator=(VideoSink&&) noexcept;
    bool valid() const;
private:
    void* handle_ = nullptr;
};

class AudioSource {
public:
    static AudioSource createDefault(uint32_t sampleRate, uint32_t channels);
    ~AudioSource();
    AudioSource(AudioSource&&) noexcept;
    AudioSource& operator=(AudioSource&&) noexcept;
    void start();
    void stop();
    bool isRunning() const;
    void addSink(AudioSink& sink);
    void removeSink(AudioSink& sink);
    bool valid() const;
private:
    void* handle_ = nullptr;
};

class AudioSink {
public:
    using Callback = std::function<void(const int16_t*, size_t, uint32_t, uint32_t)>;
    explicit AudioSink(Callback callback);
    ~AudioSink();
    AudioSink(AudioSink&&) noexcept;
    AudioSink& operator=(AudioSink&&) noexcept;
    bool valid() const;
private:
    void* handle_ = nullptr;
};

} // namespace gkit
```

---

## 9. Unit Tests

### 9.1 Rust Tests (`crates/gkit-media/tests/test_source_sink.rs`)

Referencing libwebrtc `video_broadcaster_unittest.cc`, `video_adapter_unittests.cc`, and OpenCTK frame generator tests:

| # | Test | Pattern Source |
|---|------|---------------|
| 1 | `broadcaster_add_remove_sink` | libwebrtc |
| 2 | `broadcaster_fan_out_to_multiple_sinks` | libwebrtc |
| 3 | `broadcaster_inactive_sink_does_not_receive` | libwebrtc |
| 4 | `broadcaster_wants_aggregation_or` | libwebrtc |
| 5 | `broadcaster_wants_aggregation_min` | libwebrtc |
| 6 | `broadcaster_wants_aggregation_lcm` | libwebrtc |
| 7 | `adapter_no_adapt_needed` | libwebrtc |
| 8 | `adapter_downscale_to_target` | libwebrtc |
| 9 | `adapter_rate_limit_drops_frame` | libwebrtc |
| 10 | `adapter_rate_limit_allows_first_frame` | libwebrtc |
| 11 | `adapted_source_forwards_frame_to_sinks` | libwebrtc |
| 12 | `adapted_source_drops_frame_via_adapter` | libwebrtc |
| 13 | `generator_creates_non_empty_frames` | OpenCTK |
| 14 | `generator_frames_have_correct_dimensions` | OpenCTK |
| 15 | `generator_frame_content_changes_over_time` | OpenCTK |
| 16 | `generator_stop_prevents_further_frames` | OpenCTK |
| 17 | `audio_source_produces_silence` | — |
| 18 | `audio_source_sample_rate_and_channels` | — |

### 9.2 C Tests (`apis/c/gkit-media/tests/test_source_sink.c`)

| # | Test | Description |
|---|------|-------------|
| 1 | `test_create_and_destroy` | Source+Sink lifecycle |
| 2 | `test_source_start_stop` | Start/stop without sink |
| 3 | `test_add_sink_to_source` | Sink receives frames |
| 4 | `test_remove_sink` | Removed sink gets no frames |
| 5 | `test_null_source_safety` | Null handle returns error -1 |

### 9.3 C++ Tests (`apis/cpp/gkit-media/tests/test_source_sink.cpp`)

| # | Test | Description |
|---|------|-------------|
| 1 | `VideoSourceTest.CreateDestroy` | RAII lifecycle |
| 2 | `VideoSourceTest.StartStop` | Start/stop cycle |
| 3 | `VideoSourceSink.FrameCallback` | Frames reach callback |
| 4 | `VideoSourceSink.RemoveSink` | Removed sink = no frames |
| 5 | `VideoSourceSink.MoveSemantics` | Move constructor/assignment |
| 6 | `AudioSourceTest.CreateDestroy` | RAII lifecycle |
| 7 | `AudioSourceSink.DataCallback` | Samples + silence verification |

---

## 10. Rust Example: egui SquareGenerator Viewer

Reference: `OpenCTK/src/libs/media/examples/capture/exp_square_generator.cpp`

The OpenCTK example creates a `SquareGenerator` → wraps in `FrameGeneratorCapturerVideoTrackSource` → connects to a `VideoRenderer` (SDL3-based `VideoSinkInterface<VideoFrame>`) → runs a render loop.

### 10.1 GenericKit Equivalent

Since GenericKit has an existing egui example (`crates/gkit-media/examples/gkit-media-viewer/main.rs`), the new example uses the same egui framework pattern but replaces static BMP loading with a live `VideoFrameGenerator`:

```
VideoFrameGenerator (SquarePattern, 640x480, 30fps)
    │
    ├── add_or_update_sink(egui_sink)
    │
    └── Internal thread: draw → broadcast.on_frame()
            │
            └── egui_sink.on_frame() → clone rgba → push to queue
                                            │
                                            └── egui loop: pop rgba → upload texture → display
```

### 10.2 Key Differences from OpenCTK

| OpenCTK | GenericKit (egui) |
|---------|-------------------|
| C++ `VideoRenderer` with SDL3 window | egui native window via `eframe` |
| `VideoSinkInterface<VideoFrame>` | `VideoSink<VideoFrame>` trait impl |
| Dedicated render thread with SDL events | Single-threaded egui frame loop |
| `capturer->addOrUpdateSink(renderer)` | `generator.add_or_update_sink(Box::new(sink))` |
| SDL texture upload (YUV→RGB in renderer) | egui texture from RGBA bytes |

### 10.3 Demo Layout

The egui window shows:

```
┌──────────────────────────────────────────────────┐
│  gkit-media SquareGenerator Demo                 │
├──────────────────────────────────────────────────┤
│  [▶ Start] [■ Stop]  640×480  30fps  Frame: 127 │
│                                                  │
│  ┌──────────────────────────────────┐            │
│  │                                  │            │
│  │      Live video frame            │            │
│  │      (colored squares moving     │            │
│  │       + timestamp overlay)       │            │
│  │                                  │            │
│  └──────────────────────────────────┘            │
│                                                  │
│  Pattern: ▸ SquareGenerator (default)            │
│  Output: I420 → RGBA → egui texture              │
└──────────────────────────────────────────────────┘
```

### 10.4 Implementation Details

```rust
// In main loop: receive frame from VideoFrameGenerator via shared queue
struct GeneratorSink {
    frames: Mutex<VecDeque<Vec<u8>>>,  // queued RGBA frame data
    width: u32,
    height: u32,
}

impl VideoSink<VideoFrame> for GeneratorSink {
    fn on_frame(&self, frame: &VideoFrame) {
        // Convert I420 → RGBA using existing i420_to_argb
        let mut rgba = vec![0u8; (self.width * self.height * 4) as usize];
        i420_to_argb(&frame_i420, &mut rgba, self.width * 4, VideoFormatType::Rgba);
        let mut queue = self.frames.lock().unwrap();
        if queue.len() > 2 { queue.pop_front(); }  // only keep latest
        queue.push_back(rgba);
    }
}

// egui App
struct SquareDemoApp {
    generator: VideoFrameGenerator,
    sink: Arc<GeneratorSink>,
    running: bool,
    frame_count: u64,
    texture: Option<egui::TextureHandle>,
}

impl eframe::App for SquareDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top bar: start/stop button, stats
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.running {
                    if ui.button("Stop").clicked() {
                        self.generator.stop();
                        self.running = false;
                    }
                } else {
                    if ui.button("Start").clicked() {
                        self.generator.start();
                        self.running = true;
                    }
                }
                ui.label(format!("640×480  30fps  Frame: {}", self.frame_count));
            });
        });

        // Central area: video frame
        egui::CentralPanel::default().show(ctx, |ui| {
            // Poll latest frame from sink queue
            if let Ok(mut queue) = self.sink.frames.lock() {
                if let Some(rgba) = queue.pop_back() {
                    queue.clear(); // discard older frames
                    drop(queue);
                    // Upload to egui texture
                    let size = [640.0, 480.0];
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        [640, 480], &rgba
                    );
                    let tex = ctx.load_texture("frame", color_image,
                        egui::TextureOptions::LINEAR);
                    self.texture = Some(tex);
                    self.frame_count += 1;
                }
            }
            if let Some(tex) = &self.texture {
                let available = ui.available_size();
                let scale = (available.x / 640.0).min(available.y / 480.0);
                ui.image(egui::load::SizedTexture::new(tex.id(),
                    [640.0 * scale, 480.0 * scale]));
            }
            ctx.request_repaint(); // keep egui rendering at display fps
        });
    }
}
```

### 10.5 File & Build

- **File**: `crates/gkit-media/examples/gkit-media-square-gen/main.rs` (~150 lines)
- **Build**: `cargo run -p gkit-media --example gkit-media-square-gen`
- **CMake**: Add `add_custom_target` for building + running, FOLDER `gkit_media/examples`
- **Dependencies**: `gkit-media` (local crate) + `eframe` + `egui` (already in `Cargo.toml` dev-dependencies)

### 10.6 Verification

- Run example → window shows "Start"/"Stop" button
- Click Start → colored squares appear and move + timestamp updates
- Frame counter increments at ~30fps
- Click Stop → animation pauses, frame counter stops
- Resize window → frame scales proportionally

---

## 11. Updated File Manifest

| # | File | Status | Lines (est.) |
|---|------|--------|-------------|
| 1 | `crates/gkit-media/src/webrtc/client/source_sink.rs` | NEW | ~200 |
| 2 | `crates/gkit-media/src/webrtc/client/adapter.rs` | NEW | ~120 |
| 3 | `crates/gkit-media/src/webrtc/client/mod.rs` | MODIFY | +2 lines |
| 4 | `crates/gkit-media/src/video/generator.rs` | NEW | ~250 |
| 5 | `crates/gkit-media/src/video/mod.rs` | MODIFY | +1 line |
| 6 | `apis/c/gkit-media/src/lib.rs` | MODIFY | +180 lines |
| 7 | `apis/cpp/gkit-media/gkit_media_source_sink.hpp` | NEW | ~120 |
| 8 | `apis/c/gkit-media/tests/test_source_sink.c` | NEW | ~80 |
| 9 | `apis/c/gkit-media/tests/CMakeLists.txt` | MODIFY | +10 lines |
| 10 | `apis/cpp/gkit-media/tests/test_source_sink.cpp` | NEW | ~100 |
| 11 | `apis/cpp/gkit-media/tests/CMakeLists.txt` | MODIFY | +10 lines |
| 12 | `crates/gkit-media/tests/test_source_sink.rs` | NEW | ~250 |
| 13 | `crates/gkit-media/examples/gkit-media-square-gen/main.rs` | NEW | ~150 |
| 14 | `crates/gkit-media/examples/gkit-media-square-gen/CMakeLists.txt` | NEW | ~30 |
| 15 | `crates/gkit-media/Cargo.toml` | MODIFY | +example entry |

---

## 12. Dependencies

- `std::thread`, `std::sync::Mutex`, `std::sync::Arc`, `std::sync::atomic::AtomicBool` — Rust std, no external crates (core module)
- Reuses existing `I420Buffer`, `VideoFrame`, `i420_to_argb` from `crates/gkit-media/src/video/`
- No libyuv, no tokio, no feature gate — always compiled
- `VideoFrameGenerator` pattern uses embedded 6×10 pixel bitmap font (no external file)
- egui example: `eframe` + `egui` (already in workspace `Cargo.toml` dev-dependencies)

---

## 13. Non-Goals

- Camera capture (no real device integration)
- Encoded video frames (only raw I420)
- Audio capture from microphone (only silence generator)
- Audio resampling/mixing
- Real-world time-of-day clock display (simplified timestamp string)
- Cross-compilation concerns (std-only, no platform-specific code)
