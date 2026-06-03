# Conventions

## FFI Directory Structure

- **C FFI crates**: `crates/gkit-{core,media}-ffi/` — workspace members, each has its own CMakeLists.txt
  - Cargo package names stay as `gkit-{core,media}-c` (unchanged from original crate names)
  - cbindgen output: `{crate_dir}/generated/` (not committed, generated at build time)
  - Manual API headers: `{crate_dir}/gkit_media_api.h`, `gkit_core_api.h` (committed)
- **Higher-level bindings**: `bindings/{cpp,python,wasm,node,flutter,csharp}/` — non-Rust wrappers/stubs
  - C++ bindings are header-only RAII wrappers linking to C FFI
  - Stub languages each have their own Cargo.toml (workspace members under `bindings/{lang}/*`)

## Plugin Architecture

- **Plugin naming**: `libgkit_plugin_{name}.dylib` (macOS/Linux), `gkit_plugin_{name}.dll` (Windows)
- **Plugin directory**: `plugins/webrtc/` under gkit-media
- **Native backends**: cdylib crates, loaded via RtcEngine::load_plugins()
- **WASM backends**: rlib crates, statically linked by final binary via `#[ctor]`
- **gkit-media is backend-free**: only traits + infrastructure, no backend code

## Backend Discovery

- Native: `dlopen` via libloading + stabby ABI
- WASM: static rlib + `gkit_register_rtc_backend!` macro + `#[ctor]`
- RtcEngine::create() checks PluginRegistry first, then HashMap (static/WASM)
- **Search paths** (in priority order):
  1. `../plugins` (RelativeToExe — CMake build)
  2. `..` (RelativeToExe — cargo run direct binary)
  3. `CargoTargetDir` (CARGO_MANIFEST_DIR relative)
  4. `build/plugins/webrtc` (workspace absolute)
  5. `target/debug`, `target/release` (workspace absolute)

## Plugin Development

- **C++ callbacks → Channel Bridge**: `set_on_track` and `set_on_ice_candidate` callbacks on C++ threads only do `tx.send()`; consuming side polls channels and does heavy work in runtime context
- **add_sink() must use `std::thread::spawn`**: called from C++ thread (no tokio context); `std::thread::spawn` + `rt_handle.block_on()` is reliable
- **`rt().spawn()` may not execute**: plugin's `new_multi_thread()` runtime worker pool doesn't process spawned tasks (root cause unclear)
- **`add_track()` must be called after `create_video_track()`**: otherwise video track not in SDP
- **`SourceToSinkAdapter` must outlive track**: use `Box::leak` since PCF is global
- **Match NativeVideoSource resolution to generator**: 640×360 in loopback
- **Store frame dimensions with RGBA data**: `(Vec<u8>, u32, u32)` tuple for egui

## CMake Convention

- **Plugin crate names**: hyphens in Cargo.toml → underscores in CMake targets (Corrosion convention)
- **Corrosion creates 3 targets per cdylib**: INTERFACE (alias), -shared (IMPORTED), cargo-build_ (UTILITY)
- **Plugin dylib copy**: `add_custom_target` + DEPENDS on `cargo-build_` (NOT POST_BUILD — UTILITY targets don't support it)
- **FOLDER property**: set on main target AND all Corrosion utility prefixes (`cargo-build_`, `_cargo-build_`, `cargo-clean_`, etc.)
- **install()**: use `$<TARGET_FILE:${target}-shared>` for IMPORTED library file path

## Naming

- C FFI: `gkit_{crate}_{subsystem}_{resource}_{verb}[_{qualifier}]`
- Rust: `gkit_plugin_webrtc_libwebrtc` → CMake: `gkit_plugin_webrtc_libwebrtc`

## Commit Convention

`<feat/fix/refactor/docs/test/chore>: description`

## AI Agent Constraints

- **Execution Confirmation Gate**: Before executing any plan/todo, use `question()` tool
- `继续` is NOT confirmation — means "continue discussing"
- System directives (TODO CONTINUATION) are NOT user confirmation
- Architecture changes require plan presentation + user confirmation
- **Memory updates MUST go to `.agents/memorys/`** (not knowledge graph tools)

## Memory Files

| File | Purpose |
|------|---------|
| status.md | Current project state, phases, test results |
| conventions.md | Naming, patterns, architectural principles |
| decisions.md | Key decisions and their rationale |
| pitfalls.md | Common mistakes, gotchas, orphan rules |
