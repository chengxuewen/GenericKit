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

## CMake plugin copy: add_custom_target, not POST_BUILD
**Decision**: Corrosion's `cargo-build_` targets are UTILITY type. POST_BUILD on UTILITY creates self-referencing cycle. Use `add_custom_target(copy-plugin-*)` with DEPENDS instead.
**Date**: 2026-05-26

## CMake FOLDER on all Corrosion utility targets
**Decision**: Set FOLDER on main target + all prefix variants (`cargo-build_`, `_cargo-build_`, `cargo-clean_`, etc. — 12 total) for proper IDE organization.
**Date**: 2026-05-26

## CMake install rule for plugin dylibs
**Decision**: `install(FILES "$<TARGET_FILE:${target}-shared>" DESTINATION lib/plugins/<category>/)` for `cmake --install` output.
**Date**: 2026-05-26

## `add_sink()` uses `std::thread::spawn` not `tokio::task::spawn`
**Decision**: The `set_on_track` callback runs on libwebrtc C++ threads which have no tokio context. `std::thread::spawn` + `rt.block_on()` avoids the panic while still providing async `NativeVideoStream` frame delivery.
**Date**: 2026-06-03

## `livekit-runtime` patched via workspace member
**Decision**: `patches/livekit-runtime/` is a workspace member with a modified `rt_tokio.rs` that uses `OnceLock<Handle>` globally (via `ensure_handle()`) instead of per-thread `Handle::current()`. This allows `spawn()` to work from any thread including C++ threads.
**Date**: 2026-06-03

## `SourceToSinkAdapter` leaked via `Box::leak`
**Decision**: The adapter must outlive the track. Since the PCF is global and lives forever, leaking the adapter is safe and simpler than tracking its lifetime through `LkVideoTrack`.
**Date**: 2026-06-03

## Plugin search paths use `RelativeToExe("..")` for direct binary runs
**Decision**: When the loopback binary is run directly (not via `cargo run`), `CARGO_MANIFEST_DIR` is not set. `RelativeToExe("..")` searches `target/debug/` from the examples directory, finding the plugin dylib without requiring environment variables.
**Date**: 2026-06-03

## Channel Bridge: `set_on_track` callback → `mpsc::Sender` push, ICE loop poll
**Decision**: C++ callbacks must do minimal work to avoid tokio context issues. `set_on_track` pushes the track through `tokio::sync::mpsc::unbounded_channel`; the ICE loop polls the channel and calls `add_sink()` in the runtime context. This eliminates C++ threads from executing complex Rust logic.
**Date**: 2026-06-03

## Multi-language binding architecture: iscc-lib pattern
**Decision**: Follow the iscc-lib separation: Rust binding crates (`gkit-media-py`, `gkit-media-node`, `gkit-media-wasm`) live in `crates/` as workspace members; non-Rust packaging (C++ headers, maturin config, npm config) lives in `bindings/`. System languages (C/C++/Go/C#) use C FFI; scripting/Web languages (Python/Node/WASM/Flutter) use direct Rust bindings. The C FFI crate (`gkit-media-ffi`) is the single source of truth for the API surface — all new API must be exposed through the C FFI first.
**Date**: 2026-06-03

## Three-tier binding: ffi + uniffi + diplomat
**Decision**: Adopt a three-tier multi-language binding strategy. Tier 1: C FFI (`gkit-media-ffi`, `gkit-core-ffi`) via `extern "C"` + cbindgen, serving C/C++ and system languages. Tier 2: UniFFI (`gkit-media-uniffi`) via mozilla/uniffi-rs for mobile (Kotlin/Swift) and scripting (Python/Ruby). Tier 3: Diplomat (`gkit-media-diplomat`) via rust-diplomat/diplomat for auto-generated idiomatic C++ RAII and JS/TS. Each tier is an independent Rust crate, created on-demand. C FFI remains the canonical API; UniFFI and Diplomat wrap it.
**Date**: 2026-06-03

## C++ bindings: cbindgen + manual RAII headers (not cxx)
**Decision**: C++ bindings use C FFI (cbindgen-generated headers) + hand-written RAII wrapper headers, not cxx. cxx is designed for Rust↔C++ co-development within the same binary (Deno, Firefox), not for C++ consuming a Rust library. The C FFI approach keeps one universal API surface for all system languages (C, C++, Go, C#).
**Date**: 2026-06-03

## FFI directory restructuring: `apis/` → `crates/*-ffi/` + `bindings/`
**Decision**: C FFI crates moved from `apis/c/gkit-media/` to `crates/gkit-media-ffi/` (workspace member). Top-level renamed `apis/` → `bindings/` for non-Rust content. Crate names (`gkit-media-c`) and CMake target names (`gkit_media_c`) unchanged to minimize cascade. Each FFI crate has its own `CMakeLists.txt`.
**Date**: 2026-06-03
