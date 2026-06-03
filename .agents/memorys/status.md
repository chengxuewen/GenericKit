# GenericKit Status

**Last Updated**: 2026-06-03
**Active Session**: Loopback P2P Streaming — all milestones reached ✅

## Plugin Architecture

| Phase | Status | Description |
|-------|--------|-------------|
| P0 | ✅ | gkit-core plugin loader (PluginLib, PluginLoader, PluginDiscovery) |
| P1 | ✅ | stabby types (StableVideoFrame, IStableVideoSink, IStablePeerConnectionFactory) |
| P2 | ✅ | gkit-media PluginRegistry<T> |
| P3 | ✅ | First cdylib plugin (gkit-plugin-webrtc-libwebrtc) |
| P4 | ✅ | RtcEngine PluginRegistry integration + dynamic plugin loading |
| P5 | ✅ | WASM web-sys plugin (rlib static linking) |

## Loopback P2P — Fully Working ✅

| Component | Status |
|-----------|--------|
| Plugin loads (discovery, dlopen) | ✅ |
| Backend dropdown shows libwebrtc | ✅ |
| PeerConnection creation | ✅ |
| SDP negotiation with video tracks | ✅ (`add_track()` in `create_video_track`) |
| ICE candidate exchange | ✅ |
| ICE state → Connected | ✅ |
| `set_on_track` callback fires | ✅ |
| Sender produces frames | ✅ (~390 frames/30s) |
| `add_sink()` receiver task starts | ✅ |
| **Receiver decoded frames arrive** | ✅ (~400 frames/30s — matches sender) |
| egui dual-panel display | ✅ |
| macOS `-ObjC` linker flag | ✅ `.cargo/config.toml` |

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

## Diagnostic Files (at runtime)

| File | Content |
|------|---------|
| `/tmp/gkit_loopback.log` | P2P log: SDP, ICE, events |
| `/tmp/gkit_sender_count.log` | Sender frame counter |
| `/tmp/gkit_receiver_count.log` | Receiver frame counter |
| `/tmp/gkit_rt_start.log` | Receiver task started marker |
| `/tmp/gkit_rt_frame1.log` | First receiver frame arrived |

## Test Suite

| Component | Pass | Ignored | Failed |
|-----------|------|---------|--------|
| gkit-core | 19 | 5 | 0 |
| gkit-media lib | 13 | 0 | 0 |

## Modified Files

| File | Change |
|------|--------|
| `video_track.rs` | `add_sink()` → `std::thread::spawn` + `NativeVideoStream` with unbounded queue |
| `peer_connection.rs` | `rt()` multi-thread; `add_track()`; `SourceToSinkAdapter` leak; 640×360 source |
| `factory.rs` | `rt()` call in `LiveKitRsFactory::new()` + `get_pcf()` |
| `engine.rs` | `registered_types()` includes plugins; `RelativeToExe("..")`; `build/plugins/webrtc` |
| `discovery.rs` | Error-tolerant `discover()` + recursive `scan()` |
| `registry.rs` | `names()` method |
| `loopback/main.rs` | Auto-start; file logging; frame dims; no break on Connected; `gather_complete()` |
| `.cargo/config.toml` | `-ObjC` macOS linker flag |
| `patches/livekit-runtime/` | Workspace member: `GLOBAL_HANDLE` + `set_handle()` + `ensure_handle()` |
