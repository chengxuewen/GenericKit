# WebRTC Backend Implementation Design

**Date**: 2026-05-06
**Scope**: Fill webrtc-rs backend (default), activate google_lk backend (feature-gated), extend C++ wrappers and callback system
**Constraint**: All code in single crate `gkit-media`; no new workspace members

> **P0 Prerequisite**: [VideoSource/VideoSink/VideoFrameGenerator/AudioSource](2026-05-06-video-source-sink-generator-design.md) must be implemented first. This spec references its traits (VideoSource, VideoSink) for backend integration.

---

## 1. Architecture

```
crates/gkit-media/src/
├── lib.rs                               # pub mod build_sys (uncomment for google)
├── build-sys/
│   ├── mod.rs                           # #[path = "webrtc-sys/lib.rs"] pub mod webrtc_sys;
│   └── webrtc-sys/                      # cxx.rs bridge (24 .rs, 28 .cpp, 30 .h)
├── protocols/
│   └── rtc/                             # was webrtc/
│       └── client/
│           ├── core.rs                  # trait PeerConnection, DataChannel, PeerConnectionFactory
│           ├── native/
│           │   ├── webrtc_rs.rs         # [FILL] webrtc 0.11 backend
│           │   ├── google.rs            # [REPLACE] → google_lk adapter
│           │   └── google_lk/           # LiveKit port (23 public .rs, 27 native .rs)
│           └── wasm.rs                  # web-sys stub (unchanged)
├── video/                               # protocol-agnostic media pipeline
│   ├── source_sink.rs                   # VideoSource, VideoSink, VideoBroadcaster traits
│   ├── buffer.rs / frame.rs / ...       # existing video modules
│   └── adapter.rs                       # [planned] VideoAdapter
└── capture/                             # [planned] capture sources
    └── generator.rs                     # VideoFrameGenerator
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

- VideoSource/VideoSink/AudioSource/AudioSink/VideoFrameGenerator — see separate spec
- E2EE (FrameCryptor) — google_lk code exists but not exposed
- Desktop capture — google_lk code exists but not exposed
- Stats (RTCStats) — query API exists in google_lk, not exposed to C FFI
- RTP sender/receiver/transceiver — not exposed to C FFI
- Android platform code — google_lk/android.rs not integrated
- NVIDIA/VAAPI hardware codec — code exists but requires platform-specific build
- Real-time video track rendering — VideoFrame manipulation exists; decode pipeline not in scope

---

## 10. Phase Summary

| Phase | Description | Priority | Dependencies |
|-------|-------------|----------|-------------|
| **P0** | VideoSource/Sink/Generator/Audio | P0 | None (std-only) |
| 1 | webrtc-rs backend (default) | P1 | P0 (traits) |
| 2 | google_lk backend activation | P2 | P1 |
| 3 | C++ wrappers — PeerConnection, DataChannel RAII | P1 | P1 |
| 4 | Callback system — backend → C FFI forwarding | P1 | P1 |
