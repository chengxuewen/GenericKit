# Key Decisions

## Plugin Directory: `gkit-media/plugins/`
**Decision**: Qt alignment — plugins live under the media crate, not root `plugins/`.
**Date**: 2026-05-25

## Plugin ABI: stabby + libloading (native) / inventory (WASM)
**Decision**: stabby 72.1 for cross-dylib ABI stability. WASM uses `ctor` for static registration (not `linkme`—linkme has no WASM support, not `inventory`—unnecessary complexity).
**Date**: 2026-05-25

## gkit-core plugin loader: separate from media types
**Decision**: Generic PluginLoader/PluginLib in gkit-core (like Qt's QFactoryLoader in QtCore), media-specific types/registry in gkit-media.
**Date**: 2026-05-25

## PluginBackend Drop order: instance before _lib
**Decision**: Rust drops fields in declaration order. `instance` MUST be declared before `_lib` to ensure dylib resources are freed before the library handle closes.
**Date**: 2026-05-25

## First plugin: libwebrtc via LiveKit rust-sdks
**Decision**: `gkit-media/plugins/webrtc/libwebrtc/` uses LiveKit's `webrtc-sys` + `libwebrtc` from rust-sdks (tag v0.3.34). Reuses migrated `livekit_rs` adapter code.
**Date**: 2026-05-25

## No `plugins.toml` — CMake function calls instead
**Decision**: CMake can't parse TOML natively. Use `gkit_cargo_add_plugin()` CMake function calls for plugin declarations.
**Date**: 2026-05-25

## Plugin DOES NOT depend on gkit-media (cyclic prevention)
**Decision**: gkit-media provides traits. Plugin implements traits. Neither depends on the other at build time. Final binary links both. WASM plugin auto-registers via `#[ctor]`.
**Date**: 2026-05-26

## Orphan rule: From impls → standalone functions
**Decision**: `impl From<libwebrtc::T> for gkit_media::T` violates Rust orphan rule across crate boundaries. Replaced by standalone conversion functions in `plugins/webrtc/libwebrtc/src/adapt/convert.rs`.
**Date**: 2026-05-26

## Stabby enum pattern matching not supported
**Decision**: `#[stabby::stabby]` enums don't support Rust native `match` patterns. Use `is_*()` helper methods (based on Debug format) for variant checking.
**Date**: 2026-05-26

## Test mock: register_test_backend() in lib.rs
**Decision**: Tests use `make_peer_connection()` which auto-registers a mock `PeerConnectionFactory`. No real WebRTC needed for unit/integration testing. P2P/ICE tests marked `#[ignore]`.
**Date**: 2026-05-26
