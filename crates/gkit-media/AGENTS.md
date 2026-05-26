# gkit-media AGENTS.md

**Parent:** [../../AGENTS.md](../../AGENTS.md) &mdash; root conventions, build, anti-patterns

## OVERVIEW

gkit-media is the **only active Rust crate** in GenericKit &mdash; video processing, capture, YUV color conversion, and WebRTC real-time communication.

**Plugin Architecture**: libwebrtc backend moved to `plugins/webrtc/libwebrtc/` (cdylib). gkit-media provides core traits + plugin infrastructure; backends are dynamically loaded.

## STRUCTURE

```
crates/gkit-media/
├── src/
│   ├── lib.rs               # Entry point: modules + trait re-exports
│   ├── capture/             # VideoFrameGenerator, frame patterns (SquarePattern), I420 test frames
│   ├── video/               # Video pipeline: buffers, conversion, transform, source/sink
│   │   ├── buffer.rs        # I420/I422/I444/NV12/I010 buffers, VideoBuffer trait
│   │   ├── convert.rs       # ARGB↔I420, NV12/NV21, YUY2/UYVY conversion
│   │   ├── frame.rs         # VideoFrame<T>, VideoRotation, FrameMetadata (non-stabby)
│   │   ├── frame_stabby.rs  # StableVideoFrame, VideoFrameMeta, I420Planes, NV12Planes (stabby ABI)
│   │   ├── source_sink.rs   # VideoSink/VideoSource/AudioSource traits, VideoBroadcaster
│   │   ├── transform.rs     # I420 scale/crop/rotate (90/180/270)
│   │   └── adapter.rs       # VideoAdapter (adaptive resolution/framerate)
│   ├── trait/               # Stabby ABI-stable trait definitions (cross-dylib)
│   │   ├── video_sink_stabby.rs  # IStableVideoSink
│   │   └── webrtc_stabby.rs      # IStablePeerConnectionFactory
│   ├── plugin/              # Media-specific plugin infrastructure
│   │   └── registry.rs      # PluginRegistry<T> (wraps gkit-core PluginLoader)
│   ├── protocols/rtc/       # WebRTC abstraction layer
│   │   └── client/
│   │       ├── core.rs      # PeerConnection, DataChannel, VideoTrack traits, ICE/connection states
│   │       ├── engine.rs    # RtcEngine + PluginRegistry integration + load_plugins()
│   │       ├── engine_macros.rs  # gkit_register_rtc_backend! macro
│   │       ├── wasm.rs      # Browser WebRTC backend (stub)
│   │       └── native/
│   │           ├── mod.rs        # Feature-gated module declarations
│   │           └── webrtc_rs.rs  # Real webrtc-rs backend (openh264 encode, VP8 passthrough)
│   └── build-sys/           # Feature-gated (backend-native-google)
│       └── yuv-sys/         # YUV conversion FFI (libyuv bindings)
├── plugins/                 # cdylib plugin crates (workspace members)
│   └── webrtc/libwebrtc/    # ★ libwebrtc plugin — LiveKit rust-sdks adapter
│       └── src/adapt/       #   (factory, peer_connection, data_channel, video_frame, etc.)
├── tests/                   # Integration tests
│   ├── video_frame_stabby.rs   # TDD-5: stabby VideoFrame roundtrip
│   ├── video_sink_stabby.rs    # TDD-6: IStableVideoSink CountingSink
│   └── plugin_load.rs          # Plugin dlopen + ABI check + factory creation
└── Cargo.toml               # Features: backend-native, backend-native-webrtc-rs (NO google)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add video buffer type | `src/video/buffer.rs` | Implement VideoBuffer trait |
| Add color conversion | `src/video/convert.rs` | RGB/I420/NV12/YUY2/UYVY |
| Add video transform | `src/video/transform.rs` | Scale, crop, rotate |
| Add WebRTC plugin | `plugins/webrtc/libwebrtc/src/adapt/` | cdylib crate, implement PeerConnectionFactory |
| Add stabby trait | `src/trait/` | `#[stabby::stabby(checked)]` trait definition |
| Plugin registry | `src/plugin/registry.rs` | PluginRegistry<T> wraps gkit-core loader |
| Engine integration | `src/protocols/rtc/client/engine.rs` | RtcEngine + load_plugins() |
| Add CXX FFI binding | `src/build-sys/webrtc-sys/` | Rust/C++ bridge pairs |
| Add HW encoder | `src/build-sys/webrtc-sys/{nvidia,vaapi}/` | GPU encoding |
| Add integration test | `tests/` | Each file = separate cargo test binary |
| Change feature flags | `Cargo.toml` | backend-native enables `ctor` for static registration |

## FEATURE FLAGS

| Feature | Backend | Key Deps | Notes |
|---------|---------|----------|-------|
| `backend-native` | Infrastructure | ctor | Enables plugin loading (no backend) |

**Plugin backends** (cdylib, NOT in gkit-media):
| Plugin | Location | Key Deps |
|--------|----------|----------|
| libwebrtc | `plugins/webrtc/libwebrtc/` | libwebrtc (LiveKit rust-sdks), tokio |

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
cargo test -p gkit-media --features backend-native
cargo test -p gkit-media --features backend-native
```

14 integration tests in `tests/` cover: WebRTC lifecycle, P2P, ICE, data channel, error codes, offer/answer, VideoFrame construct/convert/transform, source/sink broadcaster.

## NOTES

- **yuv-sys is nested, not a workspace member** — has its own version (0.3.14)
- **webrtc-rs backend uses tokio runtime** (spawned per PeerConnection) — ensure #[tokio::test] for async tests
- **VideoFrame ownership**: Rust VideoFrame wraps either owned I420Buffer or borrowed pointer from Google backend
