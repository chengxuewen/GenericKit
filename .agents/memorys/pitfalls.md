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

