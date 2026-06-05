# GenericKit Status

**Last Updated**: 2026-06-05
**Active Session**: WASM Loopback Frame Transmission Debug — in progress ⚠️

## Binding Architecture

| Tier | Crate | Technology | Status |
|------|-------|------------|--------|
| 1 | `gkit-media-ffi` | `extern "C"` + cbindgen | ✅ Implemented |
| 1 | `gkit-core-ffi` | `extern "C"` + cbindgen | ✅ Implemented (stub) |
| 2 | `gkit-media-wasm` | wasm-bindgen | ✅ Full WebRTC + video pipeline |
| 2 | `gkit-core-wasm` | wasm-bindgen | ✅ Version API implemented |
| 3 | `gkit-media-uniffi` | UniFFI (mozilla/uniffi-rs) | ✅ Implemented (stub) |
| 3 | `gkit-core-uniffi` | UniFFI (mozilla/uniffi-rs) | ✅ Implemented (stub) |

## WASM WebRTC Implementation (New — 2026-06-04)

| Component | Status | Details |
|-----------|--------|---------|
| `WasmPeerConnection` | ✅ | Real `web_sys::RTCPeerConnection`, all W3C methods |
| `WasmDataChannel` | ✅ | Wraps `web_sys::RtcDataChannel` |
| `WasmPeerConnectionFactory` | ✅ | ICE server config → JS objects |
| Video Track (create_video_track/set_on_track) | ✅ | Canvas capture → MediaStreamTrack |
| JS Export (gkit-media-wasm) | ✅ | 25 methods across RtcPeerConnection/RtcDataChannel/RtcVideoSource/RtcVideoSink |
| Async strategy | ✅ | `spawn_local` fire-and-forget (replaced `pollster::block_on`) |
| gmpxv | ✅ | ICE gathering polling in JS |

## Video Frame Generator Unification (New)

| Aspect | Status |
|--------|--------|
| `VideoFrameGenerator` wasm32 | ✅ `#[cfg]` branches: `setInterval` + `Closure` |
| `SquarePattern` | ✅ Shared between native and wasm32 |
| `draw_timestamp` | ✅ `js_sys::Date::now()` on wasm32, `SystemTime::now()` on native |
| Duplicate code removed | ✅ ~170 lines of manual I420 drawing deleted from `RtcVideoSource` |

## CMake WASM Build Pipeline (New)

| Target | Function |
|--------|----------|
| `cargo-build_{crate}` | cargo build --target wasm32-unknown-unknown |
| `cargo-build_{crate}_bindgen` | wasm-bindgen --target web (ALL) |
| `cargo-build_{crate}_optimize` | wasm-opt -O (ALL) |
| Auto-install | wasm-bindgen-cli + wasm-opt (brew/apt) |

## WASM Examples

| Example | Location | Status |
|---------|----------|--------|
| WebRTC P2P Loopback | `crates/gkit-media-wasm/examples/webrtc-loopback/` | ✅ Deployed to `build/examples/web/webrtc-loopback/` |
| Square Pattern Generator | `crates/gkit-media-wasm/examples/square-gen/` | ✅ Deployed to `build/examples/web/square-gen/` |

## gkit-core Conditional Compilation

| Feature | Purpose |
|---------|---------|
| `plugin` | Gates `pub mod plugin` (stabby + libloading). Default ON, OFF for wasm32 |
| `gkit-core-wasm` | Uses `default-features = false` |
| `gkit-media` wasm32 | `gkit-core = { default-features = false }` |
| 3 | `gkit-media-uniffi` | UniFFI (mozilla/uniffi-rs) | ✅ Implemented (stub) |
| 3 | `gkit-core-uniffi` | UniFFI (mozilla/uniffi-rs) | ✅ Implemented (stub) |

## Directory Structure (Final)

```
crates/                          # Rust workspace (14 crates, 6 binding)
├── gkit-media/                  # ★ Core (17K lines)
├── gkit-media-ffi/              # ★ C FFI (1168 lines)
├── gkit-media-wasm/             # WASM binding
├── gkit-media-uniffi/           # UniFFI binding
├── gkit-core/                   # Stub
├── gkit-core-ffi/               # C FFI (stub)
├── gkit-core-wasm/              # WASM binding (stub)
├── gkit-core-uniffi/            # UniFFI binding (stub)
└── gkit-{network,graphics,service,native,profiling,crash}/  # Arch stubs

packages/
└── cpp/                         # C++ RAII headers (active)
```

CMake FOLDER convention: each crate maps to a flat FOLDER matching its directory name with hyphens:

```
gkit-core              (gkit-core)
gikit-core-ffi          (gkit-core-ffi)
  gkit-core-ffi/packages/cpp
gkit-core-wasm         (gkit-core-wasm)
gkit-core-uniffi       (gkit-core-uniffi)
gkit-media             (gkit-media)
gkit-media-ffi         (gkit-media-ffi)
  gkit-media-ffi/tests
  gkit-media-ffi/examples
  gkit-media-ffi/packages/cpp  (C++ nested under FFI)
gkit-media-wasm
gkit-media-uniffi
```

Corrosion-generated targets use underscores (unavoidable per Corrosion v0.5+).

CMake options: `GKIT_BUILD_CRATE_FFI/WASM/UNIFFI`, `GKIT_BUILD_PACKAGE_CPP`.

## Plugin Architecture

| Phase | Status | Description |
|-------|--------|-------------|
| P0 | ✅ | gkit-core plugin loader (PluginLib, PluginLoader, PluginDiscovery) |
| P1 | ✅ | stabby types (StableVideoFrame, IStableVideoSink, IStablePeerConnectionFactory) |
| P2 | ✅ | gkit-media PluginRegistry<T> |
| P3 | ✅ | First cdylib plugin (gkit-plugin-webrtc-libwebrtc) |
| P4 | ✅ | RtcEngine PluginRegistry integration + dynamic plugin loading |
| P5 | ✅ | WASM web-sys plugin (rlib static linking) |

## Loopback P2P — WASM Frame Transmission Debug ⚠️

| Component | Status | Details |
|-----------|--------|---------|
| Sender frame generation | ✅ | `RtcVideoSource::start()` generates 640×360 I420 @ 15fps |
| Canvas drawing | ✅ | `CanvasSinkAdapter` I420→RGBA→offscreen→streaming canvas |
| SDP negotiation | ✅ | `[SDP offer] hasVideo=true`, offer/answer exchange works |
| **Sender WebRTC encoding** | ❌ | `hasOutboundVideo=false` — browser encoder not producing frames |
| Receiver outbound video | ❌ | `videoWidth=0`, `readyState=0`, `networkState=3` |
| Receiver frame count | ❌ | `recv=0` consistently |

### Frame Pipeline Debugging Status

| Fix Attempt | Result |
|-------------|--------|
| 1. `position:fixed` instead of `display:none` | Canvas in DOM, but still no outbound video |
| 2. Dual-canvas: putImageData→offscreen, drawImage→streaming | Compositor notified, but no outbound video |
| 3. `track.requestFrame()` after each draw | Reflect::get call works, but track may not support requestFrame |
| 4. `capture_stream_with_frame_request_rate(30.0)` | Explicit frame rate set, but no outbound video |
| 5. JS-side `<video>` for receiver | `networkState=3` (NETWORK_NO_SOURCE) — video never receives data |
| 6. Pre-play video during gesture context | srcObject set later, but video still shows no data |

### Key Diagnostic

- `[SDP offer] hasVideo=true sdpLen=5376` — SDP includes video
- `[SDP answer] hasVideo=true sdpLen=4521` — Answer includes video
- `[SND stats] hasOutboundVideo=false len=2` — **Browser WebRTC encoder never sees frames**
- `readyState=0, networkState=3` on receiver video — Decoder never receives data

**Root cause**: Canvas `captureStream()` track is registered with PeerConnection (SDP has video) but the browser's WebRTC encoder produces no outbound RTP packets. Hypothesis: `requestFrame()` may not actually be available on the track, or the canvas capture is incompatible with the browser encoder.

### Next Step

Verify whether `requestFrame()` API is available on the track. If not, explore `MediaStreamTrackGenerator` (Insertable Streams API) as alternative to canvas capture.

## Key Bug Fixes (This Session)

| # | Root Cause | Fix | File |
|---|-----------|-----|------|
| 1 | `add_track()` missing → video not in SDP | Call `add_track()` in `create_video_track()` | `peer_connection.rs` |
| 2 | `SourceToSinkAdapter` dropped → sender frames stop | `Box::leak` the adapter | `peer_connection.rs` |
| 3 | `tokio::task::spawn` on C++ thread → panic | Use `std::thread::spawn` instead | `video_track.rs` |
| 4 | Loop breaks on `Connected` → PC dropped → media stops | Remove `Connected` break condition | `loopback/main.rs` |
| 5 | Frame stride/wrong → `i420_to_argb` index OOB | Use actual frame dimensions | `loopback/main.rs` |
| 6 | Egui texture size mismatch → epaint panic | Store (data, w, h) tuple | `loopback/main.rs` |
| 7 | `registered_types()` empty for plugins | Merge plugin registry names | `engine.rs` |
| 8 | Plugin not found when running binary directly | `RelativeToExe("..")` search path | `engine.rs` |
| 9 | Single dir scan misses subdirs | Recursive `scan()` | `discovery.rs` |
| 10 | `discover()` abort on first error | Skip failed paths | `discovery.rs` |
| 11 | `NSString+StdString` crash on macOS | `-ObjC` linker flag | `.cargo/config.toml` |
| 12 | `livekit_runtime::spawn()` on C++ threads | `patches/livekit-runtime` workspace member | `Cargo.toml` |
| 13 | `set_on_track` does heavy work on C++ thread | Channel Bridge: `mpsc::Sender` push, ICE loop poll | `loopback/main.rs` |
| 14 | `rt().spawn()` doesn't execute (worker pool issue) | Keep `std::thread::spawn` + `rt_handle.block_on()` | `video_track.rs` |
| 15 | Duplicate dylibs loaded via `discover()` → ObjC conflict → SIGSEGV | Deduplicate by plugin name with `HashSet` | `discovery.rs` |
| 16 | Two separate `VideoFrameGenerator` instances → sender display ≠ actual transmitted frames | Share single `Arc<VideoFrameGenerator>` via `ArcVideoSource` wrapper; `FramePattern: Send + Sync` | `main.rs`, `generator.rs` |

## Architecture: Channel Bridge

```
C++ callback (set_on_track) → mpsc::Sender → ICE loop poll → add_sink()
                                    ↑                        ↑
                              minimal work            std::thread::spawn
                              (just tx.send)          + rt_handle.block_on
```

C++ 线程只做最轻量的 `tx.send()`，所有 Rust 逻辑在消费端线程执行。

## Diagnostic Files (at runtime)

| File | Content |
|------|---------|
| `/tmp/gkit_loopback.log` | P2P log: SDP, ICE, events |
| `/tmp/gkit_sender_count.log` | Sender frame counter |
| `/tmp/gkit_receiver_count.log` | Receiver frame counter |
| `/tmp/gkit_track_received.log` | Channel Bridge track received marker |
| `/tmp/gkit_rt_end.log` | Receiver stream ended marker |

## Test Suite

| Component | Pass | Ignored | Failed |
|-----------|------|---------|--------|
| gkit-core | 19 | 5 | 0 |
| gkit-media lib | 13 | 0 | 0 |
| Loopback P2P (30s) | sender~390 receiver~400 | — | crash:0 |

## Git State

- Branch: main, ahead of origin by multiple commits (not yet pushed)
- Working tree: clean (after binding architecture restructure)

## Remaining Work

- Push commits to origin
- Investigate `rt().spawn()` not executing (tokio worker pool issue)
- Multi-backend loopback support (webrtc-rs, WASM)
- Implement Python binding (`crates/gkit-media-py/`) when needed
- Implement Kotlin/Swift/Python export for UniFFI crates
