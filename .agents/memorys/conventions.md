# Conventions

## Language Binding Architecture (iscc-lib pattern)

All language bindings follow the **iscc-lib pattern**: Rust crates go in `crates/`, non-Rust packaging goes in `bindings/`.

### Crate Organization

```
crates/
├── gkit-media/                 # Pure Rust core (no FFI concerns)
├── gkit-media-ffi/             # C FFI (extern "C" + cbindgen) — system languages use this
├── gkit-media-py/              # Python (PyO3 — direct Rust binding)
├── gkit-media-node/            # Node.js (napi-rs — direct Rust binding)
├── gkit-media-wasm/            # WASM (wasm-bindgen — direct Rust binding)
├── gkit-media-flutter/         # Flutter (flutter_rust_bridge — direct Rust binding)
├── gkit-core/                  # Pure Rust core (stub)
├── gkit-core-ffi/              # C FFI (stub)
├── gkit-core-py/               # Python stub
└── ...

bindings/
├── cpp/gkit-media/             # C++ RAII headers (non-Rust, wraps C FFI)
├── cpp/gkit-core/
├── python/                     # maturin config, pyproject.toml (non-Rust packaging)
├── node/                       # package.json, npm config
└── ...
```

### Naming Convention

- **Rust binding crates**: `gkit-{core-crate}-{target}` where target ∈ {ffi, py, node, wasm, flutter}
  - Example: `gkit-media-py`, `gkit-core-ffi`
  - Workspace member glob: `crates/gkit-media-*`
- **C FFI crate names** (Cargo.toml `name`): historically `gkit-media-c`, `gkit-core-c` (kept for backward compatibility)
- **Non-Rust packaging**: `bindings/{lang}/` — contains build scripts, configs, but NO Cargo.toml

### Binding Strategy per Language

| Language | Crate | Strategy | Reason |
|----------|-------|----------|--------|
| **C** | `crates/gkit-media-ffi/` | `extern "C"` + cbindgen | Only way to call Rust from C |
| **C++** | `bindings/cpp/` | RAII headers on C FFI | Non-Rust, wraps C |
| **Python** | `crates/gkit-media-py/` | PyO3 (direct) | Idiomatic, better perf for video frames |
| **Node** | `crates/gkit-media-node/` | napi-rs (direct) | Idiomatic, no C FFI overhead |
| **WASM** | `crates/gkit-media-wasm/` | wasm-bindgen (direct) | Only way; WASM has no C ABI |
| **Flutter** | `crates/gkit-media-flutter/` | flutter_rust_bridge (direct) | Idiomatic Dart bindings |
| **C#** | TBD | P/Invoke on C FFI | Simple, proven pattern |
| **Go** | TBD | cgo on C FFI | Simple, proven pattern |

**Principle**: system languages (C/C++/Go/C#) → C FFI; scripting/Web languages (Python/Node/WASM/Flutter) → direct Rust bindings. C FFI serves as the universal fallback.

### C FFI as Universal Contract

- The C FFI crate (`gkit-media-ffi`) is the **single source of truth** for the API surface
- All new public API must be exposed through the C FFI first
- C tests in `crates/gkit-media-ffi/tests/` serve as the conformance test suite for all language bindings
- cbindgen headers are generated at build time into `{crate_dir}/generated/` (not committed)
- Manual API macros (`gkit_media_api.h`, `gkit_core_api.h`) are committed alongside the crate

### Three-Tier Binding Architecture

GenericKit's multi-language binding strategy uses three complementary layers:

```
gkit-media (core) ──────────────────────────────────────── 纯 Rust 逻辑
    │
    ├── gkit-media-ffi     extern "C" + cbindgen           C FFI 层
    │   → C 直接使用                                       (✅ 已实现)
    │   → C++ RAII 头文件                                  (✅ 已实现)
    │   → Go (cgo), C# (P/Invoke)                          (将来)
    │
    ├── gkit-media-uniffi  UniFFI (mozilla/uniffi-rs)      (将来)
    │   → Kotlin, Swift, Python, Ruby                      移动端 + 脚本语言
    │
    └── gkit-media-diplomat Diploma (rust-diplomat/diplomat) (将来)
        → C++ RAII（自动生成）, JS/TS, Kotlin, Dart         多语言自动生成
```

**创建时**：每个 crate 按需创建，不预建 stub。
**命名**：`gkit-{core-crate}-{ffi|uniffi|diplomat}`。
**CMake FOLDER**：`gkit_media_ffi`, `gkit_media_uniffi`, `gkit_media_diplomat`。

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
- Rust crate: `gkit-{crate}-{target}` where target ∈ {ffi, uniffi, diplomat, py, node, wasm, flutter}
- Rust plugin: `gkit_plugin_webrtc_libwebrtc` → CMake: `gkit_plugin_webrtc_libwebrtc`

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
