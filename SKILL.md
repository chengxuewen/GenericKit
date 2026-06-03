---
name: generickit-patterns
description: Coding patterns extracted from GenericKit (Rust/Cargo + CMake multi-language FFI framework)
version: 1.1.0
source: local-git-analysis
analyzed_commits: 4
analyzed_files: 200+
---

# GenericKit Patterns

## Commit Conventions

This project follows the MS-RTC commit convention with angle brackets:

- `<feat>` — New features
- `<fix>` — Bug fixes
- `<refactor>` — Refactoring
- `<docs>` — Documentation and design specs
- `<test>` — Testing
- `<chore>` — Maintenance

Note: Gerrit Change-Id hooks are enabled. Commits automatically get a Change-Id appended.

## Code Architecture

```
GenericKit/
├── Cargo.toml              # Workspace root: resolver=3, edition=2024
├── VERSION.txt             # Single source of truth for version (MAJOR.MINOR.PATCH)
├── CMakeLists.txt           # Top-level CMake: reads VERSION.txt, syncs to Cargo.toml
├── .cmake.conf              # Policy settings (CMP0075/CMP0083/CMP0091, min 3.22)
├── .clang-format            # LLVM-based, Allman braces, 120 col, 4-space indent
├── .agents/                 # AI development resources (project-local)
│   ├── rules/               # Coding standards (common + language-specific)
│   │   ├── common/          # Language-agnostic principles
│   │   ├── rust/            # Rust-specific rules
│   │   ├── cpp/             # C++ specific rules
│   │   └── ...              # Other language rules
│   └── prompts/             # Task plans and system prompts
│       ├── tasks/           # Feature task lists (webrtc-impl.md)
│       └── systems/         # System-level prompts
├── crates/                  # Internal Rust crates (10 total)
│   ├── gkit-core/           # Core library (lib + staticlib + cdylib)
│   ├── gkit-core-ffi/       # C FFI (cbindgen)
│   ├── gkit-media/          # Media/WebRTC (most developed crate)
│   ├── gkit-media-ffi/      # C FFI (cbindgen)
│   ├── gkit-native/         # Native platform integration (stub)
│   ├── gkit-crash/          # Crash handling (stub)
│   ├── gkit-network/        # Network library (stub)
│   ├── gkit-service/        # Service library (stub)
│   ├── gkit-profiling/      # Profiling (stub)
│   └── gkit-graphics/       # Graphics (stub)
├── bindings/                # Multi-language FFI bindings (6 languages)
│   ├── cpp/                 # C++ wrappers (RAII classes)
│   ├── python/              # Python (pyo3 + maturin)
│   ├── wasm/                # WebAssembly (wasm-bindgen + wasm-pack)
│   ├── node/                # Node.js (napi-rs)
│   ├── csharp/              # C# (csbindgen codegen)
│   └── flutter/             # Flutter (flutter_rust_bridge)
├── cmake/                   # CMake helper modules (16+)
├── tools/                   # CLI tools
│   ├── gkit-vcpkg/          # vcpkg helper (clap, 8 subcommands)
│   └── gkit-rc/             # gkit-rc placeholder
├── 3rdparty/                # Third-party tarballs (.tar.gz)
└── build-auto/              # Standard CMake build directory
```

## Key Patterns

### Workspace Management

- All member crates added to `[workspace].members` in root `Cargo.toml`
- `default-members` excludes crates needing special tooling (Python/maturin, Wasm/wasm-pack, Flutter)
- All crate `version` and `edition` inherit from `[workspace.package]` via `.workspace = true`
- Workspace-level dependencies declared in `[workspace.dependencies]`, referenced by path in crates

### Version Management

- Single source of truth: `VERSION.txt` (format: `MAJOR.MINOR.PATCH`)
- CMake reads `VERSION.txt` and regex-replaces `[workspace.package].version` in `Cargo.toml` at configure time
- Update flow: 1) Edit `VERSION.txt` → 2) Re-run CMake → 3) All crates inherit automatically

### CMake + Cargo (Corrosion) Integration

- Uses Corrosion (`InstallCorrosion.cmake`) as Rust-CMake bridge
- `corrosion_import_crate()` imports workspace members conditionally based on `GKIT_BUILD_API_*` options
- Each crate gets `gkit_cargo_set_folder()` for IDE organization
- All Rust build output redirected away from `build/` root via generator expressions
- Feature flags injected via `CORROSION_FEATURES` target property

### C FFI Naming Convention

Extern "C" functions follow: `gkit_<crate>_[<subsystem>]_<resource>_<verb>[_<qualifier>]`

| Function | Parse |
|----------|-------|
| `gkit_media_hello` | crate=media, no subsystem |
| `gkit_media_rtc_create_peer_connection` | crate=media, subsystem=rtc, resource=peer_connection, verb=create |
| `gkit_media_rtc_data_channel_send_text` | crate=media, subsystem=rtc, resource=data_channel, verb=send, qualifier=text |

All C types use snake_case with `_t` suffix (e.g., `gkit_media_rtc_sctp_settings_t`).

### DLL Export Macro System (Two-Layer)

**Layer 1** — `gkit_core_api.h`: Compiler-level export/import/hidden declarations
```c
#define GKIT_DECLARE_EXPORT __attribute__((visibility("default")))
#define GKIT_DECLARE_IMPORT __attribute__((visibility("default")))
```

**Layer 2** — Per-module API header (e.g., `gkit_media_api.h`): Module-level macro
```c
#ifdef GKIT_BUILD_SHARED
#   ifdef GKIT_BUILDING_MEDIA_LIB
#       define GKIT_MEDIA_API GKIT_DECLARE_EXPORT
#   else
#       define GKIT_MEDIA_API GKIT_DECLARE_IMPORT
#   endif
#else
#   define GKIT_MEDIA_API
#endif
```

### Third-Party Dependency Patterns

**Pattern A — Tarball (local .tar.gz):** GTest, Benchmark, Unity, Nuklear, ImGui
- Archive in `3rdparty/` → `cmake/FindWrap<Name>.cmake` → INTERFACE IMPORTED target `GKitWrapXXX::WrapXXX`
- `gkit_fetch_3rdparty()` extracts, `gkit_reset_dir()` cleans, `gkit_make_stamp_file()` caches

**Pattern B — vcpkg:** SDL3, etc.
- `gkit_vcpkg_install_package()` with COMPONENTS for features
- Two vcpkg copies: main + `vcpkg-tools/`

### WebRTC Backend Abstraction

Three backends behind a common trait (`core.rs`):
- `backend-native` — enables native platform support (ctor for static registration)
- `backend-wasm` — Browser-native WebRTC API

Selected via CMake cache string `GKIT_FEATURE_MEDIA_WEBRTC_BACKEND` → `CORROSION_FEATURES`.

### FFI Bindings Summary

| API | Codegen Tool | Build Tool | CMake Target Suffix |
|-----|-------------|------------|---------------------|
| C | cbindgen 0.29 | Corrosion/cargo | `_c`, `_c_static`, `_c_shared` |
| C++ | Manual RAII wrappers | CMake | `_cpp`, `_cpp_static`, `_cpp_shared` |
| Python | pyo3 | maturin | `_py_build`, `_py_develop` |
| Wasm | wasm-bindgen | wasm-pack | `_wasm_build_bundler`, `_wasm_build_web`, `_wasm_build_nodejs` |
| Node.js | napi-rs | Corrosion/cargo | `_node_build` |
| C# | csbindgen | build.rs codegen | `_csharp` |
| Flutter | flutter_rust_bridge | flutter_rust_bridge_codegen | `_flutter_bindings` |

### Testing Patterns

**Three-layer test matrix:**

| Layer | Language | Framework | Location | Count |
|-------|----------|-----------|----------|-------|
| Rust trait | Rust | `#[test]` | `crates/gkit-media/tests/` | 21 |
| C FFI | C | Unity | `crates/gkit-media-ffi/tests/` | 5 |
| C++ FFI | C++ | GTest | `bindings/cpp/gkit-media/tests/` | 1 |

**Test naming conventions:**
- Rust: `snake_case` descriptive (e.g., `create_and_close`, `error_on_closed_connection`)
- C: `test_<feature>.c` (e.g., `test_basic.c`, `test_sdp.c`, `test_video_frame.c`)
- C++: `test_<feature>.cpp`
- Tests registered as CTest via `add_test(NAME ... COMMAND ...)`
- C test targets linked with `gkit_media_c` + `GKitWrapUnity::WrapUnity`
- C++ test targets linked with `gkit_media_cpp` + `GKitWrapGTest::WrapGTest`
- FOLDER: `gkit_media_ffi/tests`, `gkit_media/bindings/cpp/tests`

**Test commands:**
```bash
cargo test -p gkit-media                          # Rust tests (native backend)
cargo test -p gkit-media --features backend-native  # All tests (mock backend)
ctest --test-dir build-auto                       # All CTest-registered tests
ctest -R gkit_media_c_test                        # C FFI tests only
```

### Code Style

- **C/C++**: `.clang-format` — LLVM base, Allman braces, 120 column, 4-space indent, PointerAlignment Right
- **C++ Standard**: configurable via CMake (C++11 default, supports up to C++26), CXX_STANDARD_REQUIRED ON
- **Rust**: `edition = "2024"` workspace-wide
- **Rust crate-type**: `["lib", "staticlib", "cdylib"]` for crates needing C linkage

### AI Development Conventions

- **`.agents/`** — Project-local AI resources (rules + prompts), read directly by AI assistants
- **`AGENTS.md`** — Entry point for AI coding assistants (build commands, architecture, key rules)
- **`SKILL.md`** — Extracted coding patterns from git history (this file)
- **Prompt files** — Stored as `prompts/*.md` (design specs) and `prompts/tasks/*.md` (feature task lists)
- Rules are project-local — no copying to `~/.claude/` required

### Adding a New Module Checklist

For each new API binding crate:
1. Add to `[workspace].members` in root `Cargo.toml`
2. Conditionally add to `_gkit_corrosion_crates` in root `CMakeLists.txt`
3. Add `gkit_cargo_set_folder()` call matching the import condition
4. Create `crates/<crate>/CMakeLists.txt` (for C FFI) or `bindings/<api>/<module>/CMakeLists.txt` (for other bindings) with target definitions
5. Create `bindings/<api>/CMakeLists.txt` (or add to root `CMakeLists.txt` for C FFI crates) with `add_subdirectory()`
6. Verify FOLDER tree: cargo-build targets are correctly grouped
7. Ensure all Rust output is redirected away from `build/` root
8. `cargo build` (default members) compiles cleanly

For a new internal Rust crate:
1. Create `crates/<crate>/Cargo.toml` with `version.workspace = true` and `edition.workspace = true`
2. Add to `[workspace.dependencies]` if referenced by other crates
3. Add to `_gkit_corrosion_crates` and `gkit_cargo_set_folder()`
4. Set `crate-type = ["lib"]` (or `["lib", "staticlib", "cdylib"]` if needed)

## Workflows

### Feature Development Flow

1. Write design spec as `prompts/<spec-name>.md`
2. Break down into tasks in `prompts/tasks/<feature>.md`
3. Implement in Rust with feature-gated compilation
4. Add tests at appropriate layer (Rust/C/C++)
5. Wire CMake options through `GKIT_OPTION_*` → `CORROSION_FEATURES`
6. Register tests in CTest
7. Verify: `cargo test`, `ctest`, `cargo check` across all feature combinations

### Version Bump

1. Edit `VERSION.txt`
2. Re-run CMake configure → auto-syncs to `Cargo.toml`
3. All crates inherit via `version.workspace = true`

### Build Automation

Standard build directory is `build-auto`:
```bash
cmake -B build-auto -S . -DGKIT_BUILD_TESTS=ON
cmake --build build-auto
ctest --test-dir build-auto
```

## CMake Helper Functions

| Function | File | Purpose |
|----------|------|---------|
| `gkit_cargo_set_folder` | `GKitCargoHelpers.cmake` | Set FOLDER for Corrosion targets |
| `gkit_cargo_setup_ffi_target` | `GKitCargoHelpers.cmake` | Create C FFI wrapper (static+shared+aggregate) |
| `gkit_cargo_setup_ffi_cpp_target` | `GKitCargoHelpers.cmake` | Create C++ FFI wrapper |
| `gkit_cargo_install_config` | `GKitCargoHelpers.cmake` | Install CMake config + pkgconfig |
| `gkit_find_package` | `GKitFindPackageHelpers.cmake` | Enhanced find_package with PROVIDED_TARGETS |
| `gkit_option` | `GKitOptionHelpers.cmake` | Enhanced option with DEPENDS/VERIFY |
| `gkit_vcpkg_install` | `InstallVcpkg.cmake` | Clone + bootstrap vcpkg |
| `gkit_vcpkg_install_package` | `InstallVcpkg.cmake` | Install + export vcpkg packages |
| `gkit_fetch_3rdparty` | `GKitCMakeHelpers.cmake` | Extract 3rdparty tarballs |
| `gkit_parse_all_arguments` | `GKitCMakeHelpers.cmake` | Strict cmake_parse_arguments |

## Internal Rust Crate Reference

| Crate | crate-type | Description |
|-------|-----------|-------------|
| `gkit-core` | lib, staticlib, cdylib | Core library |
| `gkit-media` | lib, staticlib, cdylib | Media + WebRTC (most developed) |
| `gkit-native` | lib | Native platform integration |
| `gkit-crash` | lib | Crash handling |
| `gkit-network` | lib, staticlib, cdylib | Network library |
| `gkit-service` | lib, staticlib, cdylib | Service library |
| `gkit-profiling` | lib, staticlib, cdylib | Profiling (stub) |
| `gkit-graphics` | lib, staticlib, cdylib | Graphics (stub) |
