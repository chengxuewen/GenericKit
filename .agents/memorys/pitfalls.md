# Pitfalls & Gotchas

## Rust Orphan Rule
- `impl From<ForeignType> for ForeignType` across crate boundaries → E0117
- Fix: standalone conversion functions in `convert.rs`
- Affected: 9 From impls in livekit_rs adapter migration

## Stabby Enum Pattern Matching
- `#[stabby::stabby]` enums use `#[repr(stabby)]` — not standard Rust enum layout
- `matches!()` and `match` patterns DON'T work on stabby enum variants
- Use `is_*()` predicate methods or string-based Debug check

## Cyclic Dependency: gkit-media ↔ plugin
- Plugin depends on gkit-media (for traits)
- gkit-media must NOT depend on plugin (cycle!)
- Solution: final binary links both; WASM uses `#[ctor]` auto-registration

## libwebrtc macOS SIGABRT
- libwebrtc PeerConnectionFactory init crashes in test binaries (no AppKit runloop)
- `create_peer_connection_from_plugin` test marked `#[ignore]`
- Works in real macOS app context (loopback example)

## Stabby dynptr! macro in exports
- `#[stabby::export]` + `dynptr!(Box<dyn Trait>)` has macro expansion issues
- Workaround: use raw pointer `extern "C"` functions with `Box<dyn Trait>` double-boxing

## Cargo default-members conflict
- gkit-media is in `default-members` as `crates/*`
- Plugin depends on gkit-media without features → feature unification may enable unwanted defaults
- Mitigation: workspace dep `gkit-media = { default-features = false }`

## CMake CORROSION_FEATURES for non-existent features
- Setting CORROSION_FEATURES on features that don't exist in Cargo.toml → silently ignored
- Removed all stale CORROSION_FEATURES assignments (webrtc-rs, google, wasm)

## WASM loading
- WASM32 target has no dlopen — backends must be statically linked
- `#[ctor]` with `target_arch = "wasm32"` → auto-registration on module load
- `inventory` crate was considered but `ctor` is simpler and already a dependency

## `cfg(test)` in lib code doesn't affect test binaries
- When gkit-media lib is compiled for a test binary, `cfg(test)` is FALSE
- Only the test file itself has `cfg(test)` = TRUE
- Use `#[cfg(not(test))]` to prevent plugin loading in test context

## Corrosion UTILITY targets don't support POST_BUILD
- Corrosion creates `cargo-build_` as UTILITY target (via `add_custom_target`)
- `add_custom_command(TARGET <UTILITY> POST_BUILD)` creates self-referencing cycle in CMake
- Error: `"cargo-build_X" depends on "cargo-build_X" (strong)`
- Fix: use `add_custom_target(copy-plugin-X ALL ... DEPENDS cargo-build_X)`

## CMake FOLDER set on correct target
- Corrosion creates INTERFACE library (IDE visible) + IMPORTED library (has file)
- FOLDER must be set on INTERFACE target AND cargo-build_ utility targets
- `$<TARGET_FILE:${target}-shared>` gets the actual dylib path from IMPORTED library

## GKitCargoPlugin include-once guard
- `if(DEFINED) return()` prevents function re-definition on CMake re-configure
- Fix: only guard variable init, not function definitions
- Use `if(NOT TARGET copy-plugin-X)` guards for `add_custom_target` (duplicate on re-configure)

## Loopback P2P Gotchas (2026-06-03 session)

### `SourceToSinkAdapter` must outlive the track
- `_adapter` dropped at end of `create_video_track()` → `Drop` sets `running = false` → `FrameForwarder` stops
- Fix: `Box::leak(Box::new(SourceToSinkAdapter::new(...)))` to keep alive

### `add_track()` is NOT called by `create_video_track()`
- The `PeerConnection` trait's `create_video_track()` only creates the track object
- Must call `self.inner.add_track(track, &["stream"])` for video to appear in SDP
- Without it, SDP has no `m=video` line → no media negotiation

### ICE loop breaking on Connected drops PeerConnections
- Original loopback code: `if Connected → break` → return → `pc1`/`pc2` drop → `close()` → media stops
- Fix: only break on Error or timeout; keep loop alive after Connected

### `tokio::task::spawn` panics on C++ threads
- `add_sink()` runs from `set_on_track` callback (C++ thread)
- `tokio::task::spawn()` requires `Handle::current()` which C++ threads don't have
- Fix: use `std::thread::spawn` + `rt_handle.block_on()` instead

### Egui texture dimensions must match data
- `ColorImage::from_rgba_unmultiplied([W, H], data)` — `W*H*4` must == `data.len()`
- Decoded frames may have different resolution than hardcoded constants
- Fix: store `(rgba, w, h)` tuple and use actual dimensions for texture loading

### `i420_to_argb` stride must match frame width
- `out_stride` must equal `frame.width * 4`, not a hardcoded constant
- Frame from decoder may have different width → index out of bounds

### Cargo caches compiled git deps by commit hash
- Modifying files in `~/.cargo/git/checkouts/` does NOT trigger recompilation
- Cargo checks git commit hash, not file timestamps
- Must `cargo clean` to force full rebuild (deletes 30+ GB)

### `[patch]` does not apply to path deps within git checkouts
- `libwebrtc` depends on `livekit-runtime` as `path = "../livekit-runtime"` in git repo
- Cargo `[patch."https://..."]` replaces git/crates.io sources, not path deps
- Workaround: make `patches/livekit-runtime` a workspace member + direct path dep

### `NativeVideoSource` resolution mismatch with generator
- Plugin creates source at 1280×720 but loopback generates 640×360
- Encoder may behave unexpectedly with resolution mismatch
- Fix: match source resolution to generator (640×360)

### Two `livekit-runtime` instances = two `GLOBAL_HANDLE` statics
- Plugin's path dep + `libwebrtc`'s git path dep → separate packages → separate statics
- `set_handle()` sets one instance, `spawn()` reads another → panic
- Workaround: `ensure_handle()` auto-initializes runtime on first `spawn()` call

### `rt().spawn()` doesn't execute tasks on plugin runtime
- `new_multi_thread()` + `block_on(pending())` on background thread should start worker pool
- But tasks spawned via `rt().spawn()` don't execute
- Workaround: use `std::thread::spawn` + `rt_handle.block_on()` instead
- Root cause unclear — may be tokio worker pool configuration issue

### AI Agent auto-executes without user approval
- Agent presents a plan, user says "继续" (meaning "keep discussing"), agent interprets it as approval and starts implementing code
- System directives (TODO CONTINUATION) are automated reminders, NOT user commands to start work
- Fix: always wait for explicit approval words ("执行", "批准", "go ahead") before touching code/todos/files
- Present plan → wait for approval → only then execute. Never skip the waiting step.

### Duplicate dylibs loaded → ObjC class conflict → SIGSEGV (macOS)
- `PluginDiscovery::discover()` had no dedup → same dylib found in multiple search paths loaded N times
- Each dlopen registers ObjC classes again → ObjC runtime error → segfault
- CMake + cargo builds coexist → dylib in `build/plugins/`, `build/`, `build/cargo/.../deps/`, `build/cargo/.../debug/`, `target/debug/`
- Fix: deduplicate by plugin name in `discover()` using `HashSet` (keep first occurrence)
- Also: `RelativeToExe("..")` combined with explicit `build/plugins/webrtc` path → same dylib found twice

## WASM32 Gotchas (2026-06-04 session)

### `SystemTime::now()` panics on `wasm32-unknown-unknown`
- `std::time::SystemTime::now()` wraps `js_sys::Date::now()` but `duration_since(UNIX_EPOCH).unwrap()` panics on wasm32
- Fix: use `#[cfg(target_arch = "wasm32")]` with `js_sys::Date::now() as u64` directly
- `js_sys::Date::now()` returns milliseconds since Unix epoch — divide by 1000 for seconds
- Requires `js-sys = "0.3"` in wasm32 dependencies

### `pollster::block_on` can't drive WebRTC Promises on wasm32
- WebRTC methods (`create_offer`, `set_local_description`, etc.) return JS Promises
- `pollster::block_on` only processes microtask queue, not full event loop → Promise never resolves → `unreachable` trap
- Fix: use `wasm_bindgen_futures::spawn_local` fire-and-forget + state caching
- Sink methods store results in `Rc<RefCell<Option<T>>>`; synchronous methods return cached values
- ICE candidates require polling `gathering_state()` until `"Complete"` before reading `local_description()`

### `#[cfg(feature = "plugin")]` + wasm32 feature resolution trap
- gkit-media has `plugin` feature ON (default)
- gkit-core wasm32 dep uses `default-features = false` → `plugin` OFF
- `#[cfg(feature = "plugin")]` in gkit-media evaluates to TRUE on wasm32
- But `gkit_core::plugin` doesn't exist (gating `mod plugin` in gkit-core)
- Fix: use `#[cfg(all(feature = "plugin", not(target_arch = "wasm32")))]` for gkit-core::plugin imports
- Applies to `engine.rs`, `registry.rs`, and any code importing from `gkit_core::plugin`

### `#[wasm_bindgen]` exports are snake_case, NOT camelCase
- wasm-bindgen preserves Rust snake_case for method names in JS
- JS calls must use `source.add_sink(cb)`, NOT `source.addSink(cb)`
- Same for all methods: `create_offer()`, `set_local_description()`, `add_ice_candidate()`, etc.

### CMake cargo build caching for WASM
- `cmake --build` may not detect Rust source changes → stale WASM binary
- Force rebuild: `rm -rf build-wasm && cargo clean && cmake -B build-wasm ...`
- Corrosion tracks Cargo.toml changes but not all Rust source changes reliably

### Duplicate `#[wasm_bindgen(start)]` registration
- Both `gkit-plugin-webrtc-web-sys` and `gkit-media-wasm` register wasm backend
- Registration is idempotent (`entry(key).or_insert(...)`) — duplicates are benign but the wasm module must link both
- If `gkit-plugin-webrtc-web-sys` is not in gkit-media-wasm's deps, `#[wasm_bindgen(start)]` won't fire
- Fix: gkit-media-wasm explicitly depends on the web-sys plugin

## WASM WebRTC Frame Pipeline Gotchas (2026-06-05 session)

### `captureStream()` from `display:none` canvas produces zero frames
- Elements with `display:none` have no layout box → compositor skips them
- `captureStream()` relies on compositor notifications → zero frames for `display:none` canvases
- `requestFrame()` also ineffective because compositor ignores the element entirely
- Fix: `position:fixed;left:-9999px` or `visibility:hidden` keeps the layout box

### `captureStream()` without explicit frame rate defaults to 0fps for offscreen canvases
- `canvas.captureStream()` (no args) uses browser-determined auto frame rate
- For offscreen/hidden canvases, Chrome defaults to 0fps
- Fix: use `captureStream(30)` → `capture_stream_with_frame_request_rate(30.0)` in web_sys
- API: `HtmlCanvasElement::capture_stream_with_frame_request_rate(f64)` in web_sys 0.3.97

### `video.play()` requires user gesture context for WebRTC remote tracks
- `ontrack` event fires asynchronously after user gesture expires
- `video.play()` called in ontrack handler → autoplay policy silently rejects
- Fix: call `video.play()` in the button click handler (gesture context), set `video.srcObject` later in ontrack
- `video.srcObject = stream` does NOT require user gesture

### `<video>` element with `readyState=0, networkState=3` despite live track
- `networkState=3` = `NETWORK_NO_SOURCE` — video thinks it has no data
- Even with `srcObject = new MediaStream([liveTrack])`, video may not start decoding
- More reliable: use `requestVideoFrameCallback` + Canvas for remote frame extraction in JS

### Rust WASM `add_sink` video pipeline unreliable for remote tracks
- Creating `<video>` element in Rust WASM via `spawn_local` has async timing issues
- `MediaStream::new()` + `add_track()` may not correctly associate remote track for playback
- `event.streams()[0]` from ontrack preserves original stream context
- Prefer JS-side DOM manipulation for video elements over Rust WASM

### `display:none` also blocks `<video>` decoding for remote tracks
- Hidden `<video>` elements may not trigger video decoder startup
- Pre-create video + `play()` during gesture, make temporarily visible for diagnostics
- `position:fixed` with small dimensions better than `display:none` for debugging

### Stats `getStats()` returns empty `{}` before ICE connection
- `getStats()` Promise may resolve with empty report before connection establishes
- `hasOutboundVideo=false` in stats even when SDP has `m=video`
- Stats only populate after ICE reaches "Connected" state
- First `get_stats_json()` call may fail with "stats not yet available" — retry after ICE connection

### `putImageData` alone doesn't trigger `captureStream()` compositor
- `putImageData` writes directly to bitmap, bypasses compositor
- Even with canvas in DOM (`position:fixed`), `putImageData` may not trigger frame capture
- Fix: draw to offscreen canvas via `putImageData`, then `drawImage(offscreenCanvas)` to streaming canvas
- `drawImage` goes through compositor pipeline → `captureStream()` picks it up

