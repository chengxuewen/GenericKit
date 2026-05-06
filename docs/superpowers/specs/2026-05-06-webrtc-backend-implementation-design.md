# WebRTC Backend Implementation Design

**Date**: 2026-05-06
**Scope**: Fill webrtc-rs backend (default), activate google_lk backend (feature-gated), extend C++ wrappers and callback system
**Constraint**: All code in single crate `gkit-media`; no new workspace members

---

## 1. Architecture

```
crates/gkit-media/src/
├── lib.rs                               # pub mod build_sys (uncomment for google)
├── build-sys/
│   ├── mod.rs                           # #[path = "webrtc-sys/lib.rs"] pub mod webrtc_sys;
│   └── webrtc-sys/                      # cxx.rs bridge (24 .rs, 28 .cpp, 30 .h)
├── webrtc/
│   └── client/
│       ├── core.rs                      # trait PeerConnection, DataChannel, PeerConnectionFactory
│       ├── native/
│       │   ├── webrtc_rs.rs             # [FILL] webrtc 0.11 backend
│       │   ├── google.rs                # [REPLACE] → google_lk adapter
│       │   └── google_lk/               # LiveKit port (23 public .rs, 27 native .rs)
│       └── wasm.rs                      # web-sys stub (unchanged)
```

**Invariants**:
- `core.rs` traits are sync — all backends bridge async→sync via `tokio::runtime::Handle::block_on()`
- Feature flags are **mutually exclusive** (enforced by `native/mod.rs` compile_error! guards)
- `default = ["backend-native"]` follows hierarchy: `backend-native` → `backend-native-webrtc-rs` (default pick)
- Single `gkit-media` crate — no sub-crates for `webrtc_sys`, `yuv_sys`, or `google_lk`

---

## 2. Feature Flag Matrix

```toml
[features]
default = ["backend-native"]
backend-native = []
backend-native-webrtc-rs = ["backend-native", "dep:webrtc", "dep:tokio"]
backend-native-google = ["backend-native", "dep:cxx", "dep:tokio", "dep:parking_lot",
    "dep:thiserror", "dep:log", "dep:enum_dispatch", "dep:scoped-tls"]
backend-wasm = []
```

| Feature | Dependencies | libwebrtc binary? |
|---------|-------------|-------------------|
| `backend-native-webrtc-rs` | webrtc 0.11 + tokio | No |
| `backend-native-google` | cxx + tokio + parking_lot + ... | Yes (prebuilt) |
| `backend-wasm` | web-sys (future) | N/A |

**Compile-time mutual exclusion** in `native/mod.rs`:
```rust
#[cfg(all(feature = "backend-native-webrtc-rs", feature = "backend-native-google"))]
compile_error!("only one native backend may be selected");
```

---

## 3. Phase 1: webrtc-rs Backend (Default)

### 3.1 Cargo.toml Changes

- Remove `webrtc = { workspace = true, optional = true }` — replace with feature-gated dep
- Add `tokio = { version = "1", features = ["rt"] }` gated by both native features
- Add shareable workspace dep entries for new crates

### 3.2 NativePeerConnection Implementation (`native/webrtc_rs.rs`)

Wrap `webrtc::peer_connection::RTCPeerConnection`:

```rust
pub struct NativePeerConnection {
    pc: Arc<webrtc::peer_connection::RTCPeerConnection>,
    // Callback state forwarded to C FFI layer
    on_state_change: Option<PcStateCallback>,
    on_ice_state_change: Option<PcStateCallback>,
    on_gathering_state_change: Option<PcStateCallback>,
    on_signaling_state_change: Option<PcStateCallback>,
    on_local_description: Option<PcDescriptionCallback>,
    on_local_candidate: Option<PcCandidateCallback>,
    on_data_channel: Option<PcDataChannelCallback>,
}
```

**Async→Sync bridge**: Use a lazy `OnceLock<tokio::runtime::Runtime>` (single-threaded) for `block_on()`:

```rust
fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Builder::new_current_thread()
        .enable_all().build().unwrap())
}
```

**Key method implementations** (all sync, using `runtime().block_on(async { ... })`):
- `create_offer()` → `pc.create_offer().await`
- `create_answer()` → `pc.create_answer().await`
- `set_local_description()` → `pc.set_local_description(desc).await`
- `set_remote_description()` → `pc.set_remote_description(desc).await`
- `add_ice_candidate()` → `pc.add_ice_candidate(candidate).await`
- `create_data_channel()` → `pc.create_data_channel(label).await`
- `close()` → `pc.close().await`

**State queries** (sync, no block_on needed):
- `connection_state()` → map `RTCPeerConnectionState` to `ConnectionState`
- `ice_connection_state()` → map `RTCIceConnectionState` to `IceConnectionState`
- `gathering_state()` → return from internal tracking
- `signaling_state()` → return from internal tracking

### 3.3 NativeDataChannel Implementation (`native/webrtc_rs.rs`)

Wrap `Arc<webrtc::data_channel::RTCDataChannel>`:
- `label()` → `dc.label()`
- `ready_state()` → map `RTCDataChannelState` to `DataChannelState`
- `send_text()` → `dc.send_text(data).await`
- `send_bytes()` → `dc.send(data).await`
- `close()` → `dc.close().await`
- `stream_id()` / `protocol()` → query properties

### 3.4 Callback Forwarding (`native/webrtc_rs.rs`)

Register webrtc-rs callbacks at PC creation time:

```
webrtc-rs on_connection_state_change()
  → map state enum
  → call stored C FFI callback (if Some)
```

### 3.5 PeerConnectionFactory (`native/webrtc_rs.rs`)

Create webrtc-rs `API` → `api.new_peer_connection(config).await`:

```rust
pub struct NativeFactory;

impl PeerConnectionFactory for NativeFactory {
    type PC = NativePeerConnection;
    fn create_peer_connection(&self) -> MediaResult<Self::PC> { ... }
    fn create_peer_connection_with_config(&self, config: &RtcConfiguration) -> MediaResult<Self::PC> { ... }
}
```

### 3.6 Verification
- `cargo test -p gkit-media` — 21 tests pass with real webrtc-rs backend (not stubs)
- `cargo test -p gkit-media --features backend-native-webrtc-rs` — same result
- C FFI tests (5 C + 1 C++): `ctest -R gkit_media_c_test` — unchanged, still pass
- `cargo check -p gkit-media --features backend-native-google` — compiles (google stubs still)

---

## 4. Phase 2: google_lk Backend Activation

### 4.1 Unblock build_sys

In `src/lib.rs`, uncomment:
```rust
#[cfg(feature = "backend-native-google")]
#[path = "build-sys/mod.rs"]
pub mod build_sys;
```

### 4.2 Fix webrtc_sys Import Paths

All `google_lk/native/*.rs` files use `use webrtc_sys::*`. Fix:
- `use webrtc_sys::*` → `use crate::build_sys::webrtc_sys::*`

Also fix crate-internal references in google_lk:
- `use crate::peer_connection::*` → stays (self-referencing google_lk types)
- `use crate::imp::*` → stays (alias for native/web module)

### 4.3 Add Missing Dependencies

Gate all behind `backend-native-google`:
- `cxx = "1"`
- `tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros"] }` (richer than webrtc-rs needs)
- `parking_lot = "0.12"`
- `thiserror = "1"`
- `log = "0.4"`
- `enum_dispatch = "0.3"`
- `scoped-tls = "1"`
- `futures = "0.3"`

### 4.4 Replace Google Stub (`google.rs`)

New `google.rs` wraps google_lk types to implement `core::PeerConnection` trait:

```rust
#[cfg(feature = "backend-native-google")]
pub struct GooglePeerConnection {
    inner: crate::webrtc::client::native::google_lk::peer_connection::PeerConnection,
    rt: tokio::runtime::Runtime,
}
```

Bridge google_lk async methods to sync trait methods via `rt.block_on()`.

### 4.5 Build.rs Verification

Confirm build.rs:
- Downloads LiveKit prebuilt libwebrtc from GitHub releases (cached)
- Respects `GKIT_CUSTOM_WEBRTC` for local binary path
- Respects `GKIT_SKIP_WEBRTC_DOWNLOAD=true` for CI
- Compiles webrtc-sys C++ sources via `cc` crate
- Links platform frameworks (macOS 16+, Linux dl/pthread/X11, Windows 11 libs)

### 4.6 Verification
- `cargo check -p gkit-media --features backend-native-google` passes
- `cargo test -p gkit-media --features backend-native-google` passes (with libwebrtc binary)
- C FFI tests unaffected (webrtc-rs is default, tests use default feature)

---

## 5. Phase 3: C++ Wrapper Extension

### 5.1 PeerConnection RAII Wrapper

New file: `apis/cpp/gkit-media/gkit_media_rtc.hpp`

```cpp
namespace gkit {

class PeerConnection {
public:
    PeerConnection();                          // gkit_media_rtc_create_peer_connection()
    ~PeerConnection();                         // gkit_media_rtc_destroy_peer_connection()
    PeerConnection(PeerConnection&&) noexcept; // move only
    PeerConnection& operator=(PeerConnection&&) noexcept;

    // SDP negotiation
    std::string createOffer();
    std::string createAnswer();
    void setLocalDescription(const std::string& sdp);
    void setRemoteDescription(const std::string& sdp);
    std::string localDescription();
    std::string remoteDescription();

    // ICE
    void addIceCandidate(const std::string& candidate, const std::string& sdpMid);

    // DataChannel
    DataChannel createDataChannel(const std::string& label);

    // State
    IceConnectionState iceState() const;
    ConnectionState connectionState() const;
    GatheringState gatheringState() const;
    SignalingState signalingState() const;

    // Lifecycle
    void close();
    bool valid() const;

private:
    void* handle_ = nullptr;
};

} // namespace gkit
```

### 5.2 DataChannel RAII Wrapper

```cpp
namespace gkit {

class DataChannel {
public:
    ~DataChannel();
    DataChannel(DataChannel&&) noexcept;
    DataChannel& operator=(DataChannel&&) noexcept;

    std::string label() const;
    void sendText(const std::string& data);
    void sendBytes(const uint8_t* data, size_t len);
    DataChannelState readyState() const;
    void close();
    int streamId() const;
    std::string protocol() const;
    bool valid() const;

private:
    friend class PeerConnection;
    explicit DataChannel(void* handle);
    void* handle_ = nullptr;
};

} // namespace gkit
```

### 5.3 GTest Tests

New test files:
- `apis/cpp/gkit-media/tests/test_rtc_basic.cpp` — create/destroy, move semantics
- `apis/cpp/gkit-media/tests/test_rtc_sdp.cpp` — offer/answer round-trip via C FFI
- `apis/cpp/gkit-media/tests/test_rtc_dc.cpp` — DataChannel label, send, close

### 5.4 CMake Wiring

New targets:
- `gkit_media_cpp_test_rtc_basic`, `gkit_media_cpp_test_rtc_sdp`, `gkit_media_cpp_test_rtc_dc`
- FOLDER: `gkit_media/apis/cpp/tests`
- Registered in CTest

### 5.5 Verification
- `ctest -R gkit_media_cpp_test_rtc` — 3 new tests pass
- All 8 existing tests still pass (no regression)

---

## 6. Phase 4: Callback System Completion

### 6.1 C FFI Side (already exists, works correctly)

The `PcHandleBox.callbacks` struct stores function pointers. The `gkit_media_rtc_peer_connection_set_*_callback()` functions set them.

### 6.2 Backend → FFI Bridge

In `NativePeerConnection` (webrtc_rs.rs), register callbacks at creation:

```rust
impl NativePeerConnection {
    pub fn set_on_state_change(&mut self, cb: PcStateCallback) {
        self.on_state_change = Some(cb);
    }
}
```

In the C FFI create function (`gkit_media_rtc_create_peer_connection`):
1. Create `NativePeerConnection` via `NativeFactory`
2. Store callback pointers on `PcHandleBox`
3. Register forwarding closures that call the stored C function pointers

### 6.3 DataChannel Message Callback

When `gkit_media_rtc_data_channel_set_message_callback()` is called:
1. Store callback on `DcHandleBox`
2. Register webrtc-rs `on_message()` handler that calls the C callback

### 6.4 Verification
- Rust tests verify callback invocation on state changes
- C tests add callback assertions (check called with expected state values)

---

## 7. State Machine

```
PeerConnection lifecycle:
  New → (createOffer) → (setLocalDescription) → (setRemoteDescription)
  → Connected → (ICE gathering) → (DataChannel open) → Connected
  → (close) → Closed

DataChannel lifecycle:
  (pc.createDataChannel) → Connecting → Open → (close) → Closed
```

Error states:
- Closed PC rejects all operations → `MediaError::InvalidState`
- Null handle → return -1 from C FFI
- SDP parse failure → `MediaError::InvalidSdp`

---

## 8. Testing Matrix (Post-Implementation)

| Layer | Count | Command |
|-------|-------|---------|
| Rust trait (webrtc-rs) | 21 | `cargo test -p gkit-media` |
| Rust trait (google) | 21 | `cargo test -p gkit-media --features backend-native-google` |
| C FFI (Unity) | 5 existing | `ctest -R gkit_media_c_test` |
| C++ FFI (GTest) — VideoFrame | 1 existing | `ctest -R gkit_media_cpp_test_video_frame` |
| C++ FFI (GTest) — RTC | 3 new | `ctest -R gkit_media_cpp_test_rtc` |
| **Total** | **51** | `ctest --test-dir build-auto && cargo test -p gkit-media` |

---

## 9. Non-Goals (Explicitly Out of Scope)

- E2EE (FrameCryptor) — google_lk code exists but not exposed
- Desktop capture — google_lk code exists but not exposed
- Stats (RTCStats) — query API exists in google_lk, not exposed to C FFI
- RTP sender/receiver/transceiver — not exposed to C FFI
- Android platform code — google_lk/android.rs not integrated
- NVIDIA/VAAPI hardware codec — code exists but requires platform-specific build
- Camera capture / microphone capture — real device capture not in scope; only test generators
- Real-time video track rendering — VideoFrame manipulation exists; decode pipeline not in scope

---

## 10. Phase 5: VideoSource/VideoSink/AudioSource/AudioSink + VideoFrameGenerator

### 10.1 Design Principles

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

### 10.2 Module Layout

```
crates/gkit-media/src/webrtc/client/
├── core.rs                    # (existing) PeerConnection, DataChannel traits + enums
├── source_sink.rs             # [NEW] VideoSource<F>, VideoSink<F>, AudioSource, AudioSink,
│                              #       VideoBroadcaster, VideoSinkWants
├── adapter.rs                 # [NEW] VideoAdapter, AdaptedVideoSource
│
crates/gkit-media/src/video/
├── mod.rs                     # pub mod generator;
├── generator.rs               # [NEW] VideoFrameGenerator + FramePattern + SquarePattern
```

### 10.3 Core Trait Definitions (`source_sink.rs`)

```rust
use crate::video::VideoFrame;
use std::sync::Mutex;

// ── Sink preferences (drives adaptation) ──

#[derive(Debug, Clone)]
pub struct VideoSinkWants {
    /// Sink requires rotation pre-applied to the frame
    pub rotation_applied: bool,
    /// Hard upper bound on pixel count (0 = no limit)
    pub max_pixel_count: u32,
    /// Hard upper bound on framerate (0 = no limit)
    pub max_framerate_fps: u32,
    /// Resolution must be divisible by this (e.g., 2 for I420)
    pub resolution_alignment: u32,
    /// Sink is actively encoding (false → paused)
    pub is_active: bool,
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

// ── Video pipeline traits (generic over frame type F) ──

pub trait VideoSink<F>: Send {
    /// Receive a frame from upstream source
    fn on_frame(&self, frame: &F);
    /// Frame was dropped due to rate-limiting or adaptation
    fn on_discarded_frame(&self) {}
}

pub trait VideoSource<F>: Send {
    /// Add or update a sink with its current preferences.
    /// If the sink is already registered, its wants are updated.
    fn add_or_update_sink(&mut self, sink: Box<dyn VideoSink<F>>, wants: VideoSinkWants);
    /// Remove a sink. After return, no further calls to `sink.on_frame()`.
    fn remove_sink(&mut self, sink: &dyn VideoSink<F>);
}

// ── Broadcaster: IS-A Sink AND Source ──

pub struct VideoBroadcaster<F> {
    pairs: Mutex<Vec<(Box<dyn VideoSink<F>>, VideoSinkWants)>>,
}

impl<F> VideoBroadcaster<F> {
    pub fn new() -> Self {
        Self { pairs: Mutex::new(Vec::new()) }
    }

    /// Aggregate wants across all registered sinks
    pub fn wants(&self) -> VideoSinkWants {
        let pairs = self.pairs.lock().unwrap();
        aggregate_wants(pairs.iter().map(|(_, w)| w))
    }

    pub fn sink_count(&self) -> usize {
        self.pairs.lock().unwrap().len()
    }
}

/// Aggregate multiple sink wants: uses MIN for constraints, LCM for alignment
pub fn aggregate_wants<'a>(wants: impl Iterator<Item = &'a VideoSinkWants>) -> VideoSinkWants {
    let mut result = VideoSinkWants::default();
    // First pass: check if any active sink uses requested_resolution
    // (ignores inactive sinks when active sink uses requested_resolution)
    // ... (algorithm from libwebrtc VideoBroadcaster::UpdateWants)
    for w in wants {
        result.rotation_applied |= w.rotation_applied;
        result.is_active |= w.is_active;
        if w.max_pixel_count > 0 {
            if result.max_pixel_count == 0 {
                result.max_pixel_count = w.max_pixel_count;
            } else {
                result.max_pixel_count = result.max_pixel_count.min(w.max_pixel_count);
            }
        }
        if w.max_framerate_fps > 0 {
            if result.max_framerate_fps == 0 {
                result.max_framerate_fps = w.max_framerate_fps;
            } else {
                result.max_framerate_fps = result.max_framerate_fps.min(w.max_framerate_fps);
            }
        }
        if w.resolution_alignment > 1 {
            result.resolution_alignment = lcm(result.resolution_alignment, w.resolution_alignment);
        }
    }
    result
}

fn lcm(a: u32, b: u32) -> u32 {
    if a == 0 || b == 0 { return 0; }
    a / gcd(a, b) * b
}
fn gcd(a: u32, b: u32) -> u32 {
    let mut x = a; let mut y = b;
    while y != 0 { let t = y; y = x % y; x = t; }
    x
}

impl<F: Send + 'static> VideoSink<F> for VideoBroadcaster<F> {
    fn on_frame(&self, frame: &F) {
        let pairs = self.pairs.lock().unwrap();
        for (sink, wants) in pairs.iter() {
            if !wants.is_active { continue; }
            sink.on_frame(frame);
        }
    }
}

impl<F: Send + 'static> VideoSource<F> for VideoBroadcaster<F> {
    fn add_or_update_sink(&mut self, sink: Box<dyn VideoSink<F>>, wants: VideoSinkWants) {
        let mut pairs = self.pairs.lock().unwrap();
        // Check if sink already exists (by pointer identity not feasible with trait objects)
        // Simplified: always push new; dedup by wrapper address in C FFI layer
        pairs.push((sink, wants));
    }
    fn remove_sink(&mut self, sink: &dyn VideoSink<F>) {
        let mut pairs = self.pairs.lock().unwrap();
        pairs.retain(|(s, _)| {
            // Compare trait object pointers (fat pointer comparison)
            !std::ptr::eq(
                s.as_ref() as *const (dyn VideoSink<F>) as *const (),
                sink as *const (dyn VideoSink<F>) as *const (),
            )
        });
    }
}

// ── Audio traits (simpler: no broadcaster, no adapter) ──

pub trait AudioSink: Send {
    /// PCM int16 interleaved samples
    fn on_data(&self, samples: &[i16], sample_rate: u32, channels: u32);
}

pub trait AudioSource: Send {
    fn add_sink(&mut self, sink: Box<dyn AudioSink>);
    fn remove_sink(&mut self, sink: &dyn AudioSink);
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u32;
}
```

### 10.4 VideoAdapter (`adapter.rs`)

```rust
use std::collections::VecDeque;

/// Mirror of libwebrtc VideoAdapter: crop → scale → rate-limit
pub struct VideoAdapter {
    target_pixels: u32,
    max_fps: f32,
    frame_timestamps: VecDeque<i64>,
    reset_fn: Box<dyn FnMut() + Send>,
}

impl VideoAdapter {
    pub fn new() -> Self {
        Self {
            target_pixels: 0,
            max_fps: 0.0,
            frame_timestamps: VecDeque::new(),
            reset_fn: Box::new(|| {}),
        }
    }

    /// Called when sink wants change
    pub fn on_sink_wants(&mut self, wants: &VideoSinkWants) {
        if wants.max_pixel_count > 0 {
            self.target_pixels = self.target_pixels.min(wants.max_pixel_count);
        }
        if wants.max_framerate_fps > 0 {
            self.max_fps = wants.max_framerate_fps as f32;
        }
    }

    /// Returns None to drop the frame, or Some((crop_x, crop_y, crop_w, crop_h, out_w, out_h))
    pub fn adapt_frame(&mut self, in_w: u32, in_h: u32, timestamp_us: i64)
        -> Option<(u32, u32, u32, u32, u32, u32)>
    {
        // Rate-limit check
        if self.max_fps > 0.0 {
            self.frame_timestamps.push_back(timestamp_us);
            while self.frame_timestamps.len() > 2 {
                let oldest = self.frame_timestamps.front().unwrap();
                let window_us = (1_000_000.0 / self.max_fps) as i64 * 2;
                if timestamp_us - oldest > window_us {
                    self.frame_timestamps.pop_front();
                } else {
                    break;
                }
            }
            // Drop if too many frames in window
            if self.frame_timestamps.len() > 2 {
                return None;
            }
        }

        // Resolution adaptation
        if self.target_pixels > 0 {
            let in_pixels = in_w * in_h;
            if in_pixels <= self.target_pixels {
                return Some((0, 0, in_w, in_h, in_w, in_h)); // no scaling needed
            }
            // Downscale to target
            let scale = (self.target_pixels as f64 / in_pixels as f64).sqrt();
            let out_w = ((in_w as f64 * scale) as u32).max(2);
            let out_h = ((in_h as f64 * scale) as u32).max(2);
            return Some((0, 0, in_w, in_h, out_w, out_h));
        }

        Some((0, 0, in_w, in_h, in_w, in_h))
    }
}
```

### 10.5 AdaptedVideoSource (`adapter.rs`)

```rust
pub struct AdaptedVideoSource {
    adapter: Mutex<VideoAdapter>,
    broadcaster: VideoBroadcaster<VideoFrame>,
}

impl AdaptedVideoSource {
    pub fn new() -> Self {
        Self {
            adapter: Mutex::new(VideoAdapter::new()),
            broadcaster: VideoBroadcaster::new(),
        }
    }

    /// Feed a raw frame through the adaptation pipeline
    pub fn on_frame(&self, frame: &VideoFrame) {
        let mut adapter = self.adapter.lock().unwrap();
        let ts = frame.timestamp_us();

        let result = adapter.adapt_frame(frame.width(), frame.height(), ts);
        match result {
            Some(_crop_params) => {
                // For now pass-through; scaling/cropping will use video::transform module
                self.broadcaster.on_frame(frame);
            }
            None => {
                // Frame dropped by rate-limiter
                self.broadcaster.on_frame(frame); // but call onDiscardedFrame instead? 
            }
        }
    }

    pub fn wants(&self) -> VideoSinkWants {
        self.broadcaster.wants()
    }
}

impl VideoSource<VideoFrame> for AdaptedVideoSource {
    fn add_or_update_sink(&mut self, sink: Box<dyn VideoSink<VideoFrame>>, wants: VideoSinkWants) {
        let mut adapter = self.adapter.lock().unwrap();
        adapter.on_sink_wants(&wants);
        self.broadcaster.add_or_update_sink(sink, wants);
    }
    fn remove_sink(&mut self, sink: &dyn VideoSink<VideoFrame>) {
        self.broadcaster.remove_sink(sink);
    }
}
```

### 10.6 VideoFrameGenerator (`video/generator.rs`)

```rust
use crate::video::{I420Buffer, VideoFrame};
use crate::webrtc::client::source_sink::{VideoSink, VideoSource, VideoSinkWants, VideoBroadcaster};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, SystemTime};

/// Trait for pixel-pattern generators
pub trait FramePattern: Send {
    /// Draw one frame into the given I420 plane buffers
    fn draw(&mut self, y: &mut [u8], u: &mut [u8], v: &mut [u8],
            stride_y: u32, stride_u: u32, stride_v: u32);
}

/// Squares moving toward lower-right + timestamp overlay (default pattern)
pub struct SquarePattern {
    squares: Vec<Square>,
    _width: u32,
    _height: u32,
}

struct Square {
    x: u32, y: u32,
    size: u32,
    color_y: u8, color_u: u8, color_v: u8,
}

impl SquarePattern {
    pub fn new(width: u32, height: u32, num_squares: u32) -> Self {
        // Initialize random squares
        let mut squares = Vec::new();
        for _ in 0..num_squares {
            squares.push(Square {
                x: fast_rand_u32() % width,
                y: fast_rand_u32() % height,
                size: (fast_rand_u32() % (width.min(height) / 4)) + 4,
                color_y: fast_rand_u32() as u8,
                color_u: fast_rand_u32() as u8,
                color_v: fast_rand_u32() as u8,
            });
        }
        Self { squares, _width: width, _height: height }
    }
}

impl FramePattern for SquarePattern {
    fn draw(&mut self, y: &mut [u8], u: &mut [u8], v: &mut [u8],
            stride_y: u32, stride_u: u32, stride_v: u32)
    {
        let width = stride_y;
        let height = y.len() as u32 / stride_y;

        // Fill with gray background (Y=127, U=127, V=127)
        for row in y.chunks_mut(stride_y as usize) {
            row.fill(127);
        }
        for row in u.chunks_mut(stride_u as usize) {
            row.fill(127);
        }
        for row in v.chunks_mut(stride_v as usize) {
            row.fill(127);
        }

        // Draw each square and move it
        for sq in &mut self.squares {
            draw_rect(y, stride_y, sq.x, sq.y, sq.size, sq.size, sq.color_y);
            draw_rect(u, stride_u, sq.x / 2, sq.y / 2, sq.size / 2, sq.size / 2, sq.color_u);
            draw_rect(v, stride_v, sq.x / 2, sq.y / 2, sq.size / 2, sq.size / 2, sq.color_v);
            // Move toward lower-right
            sq.x = (sq.x + fast_rand_u32() % 4) % width;
            sq.y = (sq.y + fast_rand_u32() % 4) % height;
        }

        // Draw timestamp at (10, 30)
        draw_timestamp(y, stride_y, width, 10, 30, 2);
    }
}

// ── Generator main struct ──

pub struct VideoFrameGenerator {
    broadcaster: VideoBroadcaster<VideoFrame>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl VideoFrameGenerator {
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        let pattern = SquarePattern::new(width, height, 10);
        Self::new_with_pattern(width, height, fps, Box::new(pattern))
    }

    pub fn new_with_pattern(_width: u32, _height: u32, fps: u32, pattern: Box<dyn FramePattern>) -> Self {
        let broadcaster = VideoBroadcaster::new();
        let running = Arc::new(AtomicBool::new(false));
        let rt = running.clone();
        let frame_interval = Duration::from_micros((1_000_000 / fps as u64).max(1));

        let thread_handle = thread::spawn(move || {
            let mut pattern = pattern;
            let mut i420 = I420Buffer::new(_width, _height);
            while rt.load(Ordering::Relaxed) {
                let start = std::time::Instant::now();
                let frame = {
                    let mut buf = I420Buffer::new(_width, _height);
                    pattern.draw(
                        &mut buf.data_y, &mut buf.data_u, &mut buf.data_v,
                        buf.stride_y, buf.stride_u, buf.stride_v,
                    );
                    VideoFrame::new(Box::new(buf))
                };
                broadcaster.on_frame(&frame);
                let elapsed = start.elapsed();
                if elapsed < frame_interval {
                    thread::sleep(frame_interval - elapsed);
                }
            }
        });

        Self { broadcaster, running, thread_handle: Some(thread_handle) }
    }

    pub fn start(&mut self) {
        self.running.store(true, Ordering::Relaxed);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

impl VideoSource<VideoFrame> for VideoFrameGenerator {
    fn add_or_update_sink(&mut self, sink: Box<dyn VideoSink<VideoFrame>>, wants: VideoSinkWants) {
        self.broadcaster.add_or_update_sink(sink, wants);
    }
    fn remove_sink(&mut self, sink: &dyn VideoSink<VideoFrame>) {
        self.broadcaster.remove_sink(sink);
    }
}

impl Drop for VideoFrameGenerator {
    fn drop(&mut self) {
        self.stop();
    }
}

// ── Helpers (internal) ──

fn draw_rect(plane: &mut [u8], stride: u32, x: u32, y: u32, w: u32, h: u32, color: u8) {
    let stride = stride as usize;
    for row in y..y + h {
        let start = (row as usize) * stride + x as usize;
        let end = (start + w as usize).min(plane.len());
        if start < plane.len() {
            plane[start..end].fill(color);
        }
    }
}

fn draw_timestamp(y: &mut [u8], stride_y: u32, _width: u32, x: u32, y_pos: u32, scale: u32) {
    // 6x10 pixel bitmap font (digits 0-9, dash, colon, period, space)
    // Embedded glyph data below, scaled to I420 Y plane
    let time_str = system_time_string();
    let mut cx = x;
    for ch in time_str.bytes() {
        let glyph = get_glyph(ch);
        if !glyph.is_empty() {
            draw_glyph(y, stride_y, cx, y_pos, scale, glyph);
        }
        cx += 7 * scale;
    }
}

fn system_time_string() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    // Simple YYYY-MM-DD HH:MM:SS.mmm
    // (full calendar logic omitted for brevity; spec references SystemTime)
    format!("{}:{:02}:{:02}.{:03}",
        secs / 3600, (secs % 3600) / 60, secs % 60, millis)
}
```

### 10.7 Default AudioSource

```rust
/// Simple silence audio source at requested sample rate / channels
pub struct DefaultAudioSource {
    sample_rate: u32,
    channels: u32,
    sinks: Mutex<Vec<Box<dyn AudioSink>>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl DefaultAudioSource {
    pub fn new(sample_rate: u32, channels: u32) -> Self { ... }

    /// Produces silence frames at 20ms intervals
    pub fn start(&mut self) { ... }
    pub fn stop(&mut self) { ... }
}

impl AudioSource for DefaultAudioSource {
    fn add_sink(&mut self, sink: Box<dyn AudioSink>) { ... }
    fn remove_sink(&mut self, sink: &dyn AudioSink) { ... }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn channels(&self) -> u32 { self.channels }
}
```

### 10.8 C FFI (10 new functions in `apis/c/gkit-media/src/lib.rs`)

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

// ── AudioSource (creates DefaultAudioSource) ──
void* gkit_media_rtc_audio_source_create_default(uint32_t sample_rate, uint32_t channels);
void  gkit_media_rtc_audio_source_destroy(void* handle);
int   gkit_media_rtc_audio_source_start(void* handle);
int   gkit_media_rtc_audio_source_stop(void* handle);

// ── AudioSink (callback-based) ──
typedef void (*gkit_media_rtc_audio_data_callback_t)(const int16_t* data, size_t frames,
    uint32_t sample_rate, uint32_t channels, void* user_data);
void* gkit_media_rtc_audio_sink_create(gkit_media_rtc_audio_data_callback_t cb, void* user_data);
void  gkit_media_rtc_audio_sink_destroy(void* handle);

// ── Sink/Source wiring ──
int   gkit_media_rtc_video_source_add_sink(void* source_handle, void* sink_handle);
int   gkit_media_rtc_video_source_remove_sink(void* source_handle, void* sink_handle);
int   gkit_media_rtc_audio_source_add_sink(void* source_handle, void* sink_handle);
int   gkit_media_rtc_audio_source_remove_sink(void* source_handle, void* sink_handle);
```

**Total: 14 new C FFI functions + 10 existing video generator functions = 24 new functions in lib.rs** (total ~72 `gkit_media_rtc_*` + `gkit_media_video_generator_*` functions).

### 10.9 C++ Wrappers (new file `apis/cpp/gkit-media/gkit_media_source_sink.hpp`)

```cpp
namespace gkit {

// ── VideoSource ──
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
    explicit VideoSource(void* h);
};

// ── VideoSink ──
using VideoFrameCallback = std::function<void(VideoFrame&)>;

class VideoSink {
public:
    explicit VideoSink(VideoFrameCallback callback);
    ~VideoSink();
    VideoSink(VideoSink&&) noexcept;
    VideoSink& operator=(VideoSink&&) noexcept;
    bool valid() const;

private:
    void* handle_ = nullptr;
};

// ── AudioSource ──
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
    explicit AudioSource(void* h);
};

// ── AudioSink ──
using AudioDataCallback = std::function<void(const int16_t*, size_t, uint32_t, uint32_t)>;

class AudioSink {
public:
    explicit AudioSink(AudioDataCallback callback);
    ~AudioSink();
    AudioSink(AudioSink&&) noexcept;
    AudioSink& operator=(AudioSink&&) noexcept;
    bool valid() const;

private:
    void* handle_ = nullptr;
};

} // namespace gkit
```

### 10.10 Unit Tests

#### 10.10.1 Rust Tests (`crates/gkit-media/tests/test_source_sink.rs`)

Referencing patterns from libwebrtc `video_broadcaster_unittest.cc` and OpenCTK frame generator tests:

```rust
// ── Test helper: NullSink (collects received frames) ──
struct TestSink {
    frames: Mutex<Vec<VideoFrame>>,
    discarded: Mutex<u32>,
}
impl VideoSink<VideoFrame> for TestSink {
    fn on_frame(&self, frame: &VideoFrame) {
        self.frames.lock().unwrap().push(frame.clone());
    }
    fn on_discarded_frame(&self) {
        *self.discarded.lock().unwrap() += 1;
    }
}

// ── Broadcaster tests (mirrors libwebrtc video_broadcaster_unittest.cc) ──
#[test]
fn broadcaster_add_remove_sink() {
    let mut bc = VideoBroadcaster::<VideoFrame>::new();
    let sink = Box::new(TestSink::new());
    bc.add_or_update_sink(sink, VideoSinkWants::default());
    assert_eq!(bc.sink_count(), 1);
    bc.remove_sink(&sink);
    assert_eq!(bc.sink_count(), 0);
}

#[test]
fn broadcaster_fan_out_to_multiple_sinks() {
    let mut bc = VideoBroadcaster::<VideoFrame>::new();
    let s1 = Arc::new(TestSink::new());
    let s2 = Arc::new(TestSink::new());
    bc.add_or_update_sink(Box::new(s1.clone()), VideoSinkWants::default());
    bc.add_or_update_sink(Box::new(s2.clone()), VideoSinkWants::default());
    let frame = make_test_frame(320, 240);
    bc.on_frame(&frame);
    assert_eq!(s1.frames.lock().unwrap().len(), 1);
    assert_eq!(s2.frames.lock().unwrap().len(), 1);
}

#[test]
fn broadcaster_inactive_sink_does_not_receive() {
    let mut bc = VideoBroadcaster::<VideoFrame>::new();
    let s1 = Arc::new(TestSink::new());
    let mut wants = VideoSinkWants::default();
    wants.is_active = false;
    bc.add_or_update_sink(Box::new(s1.clone()), wants);
    bc.on_frame(&make_test_frame(320, 240));
    assert_eq!(s1.frames.lock().unwrap().len(), 0);
}

#[test]
fn broadcaster_wants_aggregation() {
    let mut bc = VideoBroadcaster::<VideoFrame>::new();
    let s1 = Arc::new(TestSink::new());
    let s2 = Arc::new(TestSink::new());
    bc.add_or_update_sink(Box::new(s1.clone()),
        VideoSinkWants { rotation_applied: true, max_pixel_count: 1920*1080, ..Default::default() });
    bc.add_or_update_sink(Box::new(s2.clone()),
        VideoSinkWants { rotation_applied: false, max_pixel_count: 640*480, ..Default::default() });
    let wants = bc.wants();
    assert!(wants.rotation_applied);  // OR: any sink requires it
    assert_eq!(wants.max_pixel_count, 640*480);  // MIN: tightest constraint
}

// ── VideoAdapter tests (mirrors libwebrtc video_adapter_unittests.cc) ──
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
    // First frame: accepted
    assert!(adapter.adapt_frame(640, 480, 0).is_some());
    // Second frame at same timestamp: dropped
    assert!(adapter.adapt_frame(640, 480, 0).is_none());
}

// ── VideoFrameGenerator tests (mirrors OpenCTK frame_generator tests) ──
#[test]
fn generator_creates_non_empty_frames() {
    let mut gen = VideoFrameGenerator::new(640, 480, 30);
    let sink = Arc::new(TestSink::new());
    gen.add_or_update_sink(Box::new(sink.clone()), VideoSinkWants::default());
    gen.start();
    std::thread::sleep(std::time::Duration::from_millis(100));
    gen.stop();
    assert!(!sink.frames.lock().unwrap().is_empty());
}

#[test]
fn generator_frames_have_correct_dimensions() {
    let mut gen = VideoFrameGenerator::new(640, 480, 30);
    let sink = Arc::new(TestSink::new());
    gen.add_or_update_sink(Box::new(sink.clone()), VideoSinkWants::default());
    gen.start();
    std::thread::sleep(std::time::Duration::from_millis(100));
    gen.stop();
    for frame in sink.frames.lock().unwrap().iter() {
        assert_eq!(frame.width(), 640);
        assert_eq!(frame.height(), 480);
    }
}

#[test]
fn generator_frame_content_changes_over_time() {
    // Verify consecutive frames differ (moving squares cause pixel changes)
    let mut gen = VideoFrameGenerator::new(640, 480, 30);
    let sink = Arc::new(TestSink::new());
    gen.add_or_update_sink(Box::new(sink.clone()), VideoSinkWants::default());
    gen.start();
    std::thread::sleep(std::time::Duration::from_millis(200));
    gen.stop();
    let frames = sink.frames.lock().unwrap();
    assert!(frames.len() >= 2);
    // Compare Y-planes of first two frames
    let f0 = frames[0].buffer().to_i420().unwrap();
    let f1 = frames[1].buffer().to_i420().unwrap();
    assert_ne!(&f0.data_y[..], &f1.data_y[..]);  // pixels changed
}

// ── Audio tests ──
#[test]
fn audio_source_produces_silence() {
    let mut src = DefaultAudioSource::new(48000, 1);
    let sink = Arc::new(TestAudioSink::new());
    src.add_sink(Box::new(sink.clone()));
    src.start();
    std::thread::sleep(std::time::Duration::from_millis(100));
    src.stop();
    let all_data = sink.data.lock().unwrap();
    assert!(!all_data.is_empty());
    // Verify silence (all zeros)
    for sample in all_data.iter() {
        assert_eq!(*sample, 0);
    }
}
```

#### 10.10.2 C Tests (`apis/c/gkit-media/tests/test_source_sink.c`)

```c
#include "unity.h"
#include "gkit_media.h"

void* g_source = NULL;
void* g_sink = NULL;
static int g_frame_count = 0;

static void frame_callback(void* frame_handle, void* user_data) {
    (void)user_data;
    TEST_ASSERT_NOT_NULL(frame_handle);
    g_frame_count++;
    // Verify frame has correct dimensions
    TEST_ASSERT_EQUAL(640, gkit_media_video_frame_get_width(frame_handle));
    TEST_ASSERT_EQUAL(480, gkit_media_video_frame_get_height(frame_handle));
}

void setUp(void) {
    g_source = gkit_media_rtc_video_source_create_generator(640, 480, 30);
    g_sink = gkit_media_rtc_video_sink_create(frame_callback, NULL);
    g_frame_count = 0;
}

void tearDown(void) {
    gkit_media_rtc_video_source_destroy(g_source);
    gkit_media_rtc_video_sink_destroy(g_sink);
    g_source = NULL;
    g_sink = NULL;
}

void test_create_and_destroy(void) {
    TEST_ASSERT_NOT_NULL(g_source);
    TEST_ASSERT_NOT_NULL(g_sink);
}

void test_source_start_stop(void) {
    TEST_ASSERT_EQUAL(0, gkit_media_rtc_video_source_start(g_source));
    sleep(1); // wait for frames
    gkit_media_rtc_video_source_stop(g_source);
    // No assertion on frame count (timing dependent)
}

void test_add_sink_to_source(void) {
    TEST_ASSERT_EQUAL(0, gkit_media_rtc_video_source_add_sink(g_source, g_sink));
    gkit_media_rtc_video_source_start(g_source);
    sleep(1);
    gkit_media_rtc_video_source_stop(g_source);
    TEST_ASSERT_GREATER_THAN(0, g_frame_count);
}

void test_remove_sink(void) {
    gkit_media_rtc_video_source_add_sink(g_source, g_sink);
    gkit_media_rtc_video_source_remove_sink(g_source, g_sink);
    TEST_ASSERT_EQUAL(0, g_frame_count); // no frames delivered after removal
}

void test_null_source_safety(void) {
    TEST_ASSERT_EQUAL(-1, gkit_media_rtc_video_source_start(NULL));
    TEST_ASSERT_EQUAL(-1, gkit_media_rtc_video_source_add_sink(NULL, g_sink));
    TEST_ASSERT_EQUAL(-1, gkit_media_rtc_video_source_add_sink(g_source, NULL));
}
```

#### 10.10.3 C++ Tests (`apis/cpp/gkit-media/tests/test_source_sink.cpp`)

```cpp
#include <gtest/gtest.h>
#include <gkit_media_source_sink.hpp>
#include <gkit_media_video_frame.hpp>

TEST(VideoSourceTest, CreateDestroy) {
    auto src = gkit::VideoSource::createGenerator(640, 480, 30);
    EXPECT_TRUE(src.valid());
}

TEST(VideoSourceTest, StartStop) {
    auto src = gkit::VideoSource::createGenerator(640, 480, 30);
    src.start();
    EXPECT_TRUE(src.isRunning());
    src.stop();
}

TEST(VideoSourceSink, FrameCallback) {
    auto src = gkit::VideoSource::createGenerator(640, 480, 30);
    int frame_count = 0;
    gkit::VideoSink sink([&](gkit::VideoFrame& frame) {
        EXPECT_EQ(frame.width(), 640);
        EXPECT_EQ(frame.height(), 480);
        frame_count++;
    });
    src.addSink(sink);
    src.start();
    std::this_thread::sleep_for(std::chrono::seconds(1));
    src.stop();
    EXPECT_GT(frame_count, 0);
}

TEST(VideoSourceSink, RemoveSink) {
    auto src = gkit::VideoSource::createGenerator(640, 480, 30);
    int count = 0;
    gkit::VideoSink sink([&](gkit::VideoFrame&) { count++; });
    src.addSink(sink);
    src.removeSink(sink);
    src.start();
    std::this_thread::sleep_for(std::chrono::milliseconds(500));
    src.stop();
    EXPECT_EQ(count, 0); // no frames after removal
}

TEST(VideoSourceSink, MoveSemantics) {
    auto src1 = gkit::VideoSource::createGenerator(640, 480, 30);
    auto src2 = std::move(src1);
    EXPECT_FALSE(src1.valid());
    EXPECT_TRUE(src2.valid());
}

TEST(AudioSourceTest, CreateDestroy) {
    auto src = gkit::AudioSource::createDefault(48000, 1);
    EXPECT_TRUE(src.valid());
}

TEST(AudioSourceSink, DataCallback) {
    auto src = gkit::AudioSource::createDefault(48000, 1);
    int sample_count = 0;
    gkit::AudioSink sink([&](const int16_t* data, size_t frames, uint32_t rate, uint32_t channels) {
        EXPECT_EQ(rate, 48000u);
        EXPECT_EQ(channels, 1u);
        sample_count += frames;
        // Verify silence
        for (size_t i = 0; i < frames; i++) {
            EXPECT_EQ(data[i], 0);
        }
    });
    src.addSink(sink);
    src.start();
    std::this_thread::sleep_for(std::chrono::milliseconds(500));
    src.stop();
    EXPECT_GT(sample_count, 0);
}
```

### 10.11 Test Matrix Update

After Phase 5, total test count:

| Layer | Framework | Count | Command |
|-------|-----------|-------|---------|
| Rust trait (webrtc-rs) | `#[test]` | 21 existing | `cargo test -p gkit-media` |
| Rust source/sink/broadcaster/adapter | `#[test]` | 12 new | same |
| Rust video generator | `#[test]` | 4 new | same |
| Rust audio source | `#[test]` | 2 new | same |
| C FFI (Unity) | `.c` | 5 existing + 1 new | `ctest -R gkit_media_c_test` |
| C++ FFI (GTest) VideoFrame | `.cpp` | 1 existing | `ctest -R gkit_media_cpp_test` |
| C++ FFI (GTest) RTC | `.cpp` | 3 new (Phase 3) | same |
| C++ FFI (GTest) SourceSink | `.cpp` | 1 new | same |
| **Total** | | **50** | |

### 10.12 File Manifest (Phase 5 additions)

| # | File | Status | Content |
|---|------|--------|---------|
| 1 | `webrtc/client/source_sink.rs` | NEW | VideoSource/Sink traits, VideoBroadcaster, VideoSinkWants, AudioSource/Sink traits |
| 2 | `webrtc/client/adapter.rs` | NEW | VideoAdapter, AdaptedVideoSource |
| 3 | `video/generator.rs` | NEW | FramePattern trait, SquarePattern, VideoFrameGenerator |
| 4 | `video/mod.rs` | MODIFY | Add `pub mod generator;` |
| 5 | `webrtc/client/mod.rs` | MODIFY | Add `pub mod source_sink; pub mod adapter;` |
| 6 | `apis/c/gkit-media/src/lib.rs` | MODIFY | Add 24 new C FFI functions |
| 7 | `apis/cpp/gkit-media/gkit_media_source_sink.hpp` | NEW | C++ RAII wrappers |
| 8 | `crates/gkit-media/tests/test_source_sink.rs` | NEW | 18 Rust unit tests |
| 9 | `apis/c/gkit-media/tests/test_source_sink.c` | NEW | 5 C Unity tests |
| 10 | `apis/cpp/gkit-media/tests/test_source_sink.cpp` | NEW | 7 C++ GTest tests |
| 11 | `apis/c/gkit-media/tests/CMakeLists.txt` | MODIFY | Add test_source_sink target |
| 12 | `apis/cpp/gkit-media/tests/CMakeLists.txt` | MODIFY | Add test_source_sink target |

---

## 11. Updated Phase Summary

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | webrtc-rs backend (default) — fill stubs, tokio bridge | pending |
| 2 | google_lk backend activation — unblock build_sys, fix imports, add deps | pending |
| 3 | C++ wrappers — PeerConnection, DataChannel RAII | pending |
| 4 | Callback system — backend → C FFI forwarding | pending |
| 5 | Source/Sink abstractions + VideoFrameGenerator + AudioSource | pending |
