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

- 9 commits on main (unchanged from before), ~2 commits pending (Channel Bridge + cleanup)
- Working tree: clean

## Remaining Work

- Investigate `rt().spawn()` not executing (tokio worker pool issue)
- Clean up cached `~/.cargo/git/...` livekit-runtime modifications (redundant after `[patch]`)
- Multi-backend loopback support (webrtc-rs, WASM)
