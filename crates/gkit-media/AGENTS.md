# gkit-media AGENTS.md

**Parent:** [../../AGENTS.md](../../AGENTS.md) &mdash; root conventions, build, anti-patterns

## OVERVIEW

gkit-media is the **only active Rust crate** in GenericKit &mdash; video processing, capture, YUV color conversion, and WebRTC real-time communication. ~17K lines (103 Rust files + 98 C++/ObjC files).

## STRUCTURE

```
crates/gkit-media/
├── src/
│   ├── lib.rs               # Entry point: 3 public modules + build_sys, 2 public fns
│   ├── capture/             # VideoFrameGenerator, frame patterns (SquarePattern), I420 test frames
│   ├── video/               # Video pipeline: buffers, conversion, transform, source/sink
│   │   ├── buffer.rs        # I420/I422/I444/NV12/I010 buffers, VideoBuffer trait
│   │   ├── convert.rs       # ARGB↔I420, NV12/NV21, YUY2/UYVY conversion
│   │   ├── frame.rs         # VideoFrame<T>, VideoRotation, FrameMetadata
│   │   ├── source_sink.rs   # VideoSink/VideoSource/AudioSource traits, VideoBroadcaster
│   │   ├── transform.rs     # I420 scale/crop/rotate (90/180/270)
│   │   └── adapter.rs       # VideoAdapter (adaptive resolution/framerate)
│   ├── protocols/rtc/       # WebRTC abstraction layer
│   │   └── client/
│   │       ├── core.rs      # PeerConnection, DataChannel, VideoTrack traits, ICE/connection states
│   │       ├── engine.rs    # Plugin backend registry (RtcEngine)
│   │       ├── engine_macros.rs  # gkit_register_rtc_backend! macro
│   │       ├── wasm.rs      # Browser WebRTC backend (stub)
│   │       └── native/
│   │           ├── webrtc_rs.rs     # Real webrtc-rs backend (openh264 encode, VP8 passthrough)
│   │           ├── google.rs        # Google libwebrtc stub
│   │           └── google_lk/       # Full LiveKit bindings (25 modules)
│   │               ├── native/      # CXX FFI bridge: video_frame, yuv_helper, peer_connection
│   │               └── web/         # WASM bridge
│   └── build-sys/           # Feature-gated (backend-native-google)
│       ├── webrtc-sys/      # 28 Rust FFI + 98 C++/ObjC files for Google libwebrtc
│       │   ├── nvidia/      # NVENC/NVDEC GPU encoder/decoder
│       │   ├── vaapi/       # VAAPI Linux hardware encoder
│       │   └── include/     # LiveKit C++ bridge headers
│       └── yuv-sys/         # YUV conversion FFI (libyuv bindings)
├── tests/                   # 14 Rust integration tests (WebRTC + VideoFrame)
├── examples/                # viewer, square-gen, webrtc-loopback
└── Cargo.toml               # 14 optional deps, 3 feature flags
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add video buffer type | `src/video/buffer.rs` | Implement VideoBuffer trait |
| Add color conversion | `src/video/convert.rs` | RGB/I420/NV12/YUY2/UYVY |
| Add video transform | `src/video/transform.rs` | Scale, crop, rotate |
| Add WebRTC backend | `src/protocols/rtc/client/` | Register via RtcEngine + macro |
| Change webrtc-rs backend | `src/protocols/rtc/client/native/webrtc_rs.rs` | 363 lines, H264 encode |
| Change Google backend | `src/protocols/rtc/client/native/google_lk/` | 25 modules, CXX FFI |
| Add CXX FFI binding | `src/build-sys/webrtc-sys/` | Rust/C++ bridge pairs |
| Add HW encoder | `src/build-sys/webrtc-sys/{nvidia,vaapi}/` | GPU encoding |
| Add integration test | `tests/` | Each file = separate cargo test binary |
| Change feature flags | `Cargo.toml` | backend-native-google (default), backend-native-webrtc-rs |

## FEATURE FLAGS

| Feature | Backend | Key Deps | Notes |
|---------|---------|----------|-------|
| `backend-native-google` (default) | Google libwebrtc via CXX FFI | cxx, tokio, parking_lot, thiserror, serde | Requires C++ toolchain |
| `backend-native-webrtc-rs` | Pure Rust webrtc crate | webrtc 0.17, tokio, bytes, openh264 | No C++ needed |
| `backend-native-all` | Both native backends | Both of the above + ctor | For testing all |

**CMake selects backend**: `GKIT_FEATURE_MEDIA_WEBRTC_BACKEND` ∈ {`webrtc-rs`, `google`, `wasm`}. WASM targets lock to `wasm` backend.

## ARCHITECTURE

```
Video Pipeline:
  capture/generator.rs → video/frame.rs → video/transform.rs → video/buffer.rs
                          ↕ (convert.rs)

WebRTC Stack:
  protocols/rtc/client/core.rs (traits)
    ├── protocols/rtc/client/engine.rs (registry)
    ├── native/webrtc_rs.rs (pure Rust backend)
    ├── native/google_lk/native/ (CXX FFI → Google libwebrtc)
    └── wasm.rs (browser WebRTC, stub)
```

H264 encode path (webrtc-rs): `openh264::encoder::Encoder` → `RtpVideoFrame::H264` → `Track::from_video_frame`
Google backend: `VideoFrame::to_webrtc_frame()` → CXX bridge → libwebrtc C++ VideoFrame

## TESTING

```bash
# All Rust tests (select backend)
cargo test -p gkit-media --features backend-native-webrtc-rs
cargo test -p gkit-media --features backend-native-google

# Single integration test
cargo test -p gkit-media webrtc_basic -- --nocapture

# Unit tests only
cargo test -p gkit-media --lib
```

14 integration tests in `tests/` cover: WebRTC lifecycle, P2P, ICE, data channel, error codes, offer/answer, VideoFrame construct/convert/transform, source/sink broadcaster.

## NOTES

- **170+ unsafe blocks** in `google_lk/native/` and `build-sys/webrtc-sys/` — many missing `// SAFETY:` comments
- **yuv-sys is nested, not a workspace member** — has its own version (0.3.14)
- **build-sys/** is 23K lines (mostly NVIDIA/VAAPI C++) — gated behind `backend-native-google`
- **webrtc-rs backend uses tokio runtime** (spawned per PeerConnection) — ensure #[tokio::test] for async tests
- **VideoFrame ownership**: Rust VideoFrame wraps either owned I420Buffer or borrowed pointer from Google backend
