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
