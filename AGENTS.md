# GenericKit AGENTS.md

**Generated:** 2026-05-25
**Commit:** edf4251
**Branch:** main

## OVERVIEW

GenericKit is a multi-language development toolkit providing algorithms, data structures, media processing, and FFI bindings. Core stack: Rust (edition 2024, workspace), CMake (via Corrosion 0.6.1 for Rust integration), C/C++ (cbindgen, GTest, Unity).

## STRUCTURE

```
GenericKit/
├── crates/              # Rust workspace crates (8 total, 1 active)
│   ├── gkit-media/      # ★ ACTIVE — video processing + WebRTC (~17K lines)
│   ├── gkit-core/       # STUB — core primitives (to be implemented)
│   ├── gkit-network/    # STUB — networking
│   ├── gkit-graphics/   # STUB — graphics
│   ├── gkit-service/    # STUB — background services
│   ├── gkit-native/     # STUB — OS platform abstraction
│   ├── gkit-profiling/  # STUB — profiling/tracing
│   └── gkit-crash/      # STUB — crash reporting
├── apis/                # FFI bindings (7 languages × 2 crates)
│   ├── c/               # C FFI (extern "C" + cbindgen) — working
│   ├── cpp/             # C++ RAII wrappers on C FFI
│   ├── python/          # PyO3 + maturin — stub
│   ├── wasm/            # wasm-bindgen — stub
│   ├── node/            # napi-rs — stub
│   ├── flutter/         # flutter_rust_bridge — stub
│   └── csharp/          # csbindgen — scaffold (broken)
├── cmake/               # CMake build modules (19 files)
├── tools/               # CLI tools
│   ├── gkit-vcpkg/      # vcpkg integration CLI
│   └── gkit-rc/         # Resource compiler (stub)
├── 3rdparty/            # Local .tar.gz archives (Corrosion, GTest, Unity, etc.)
├── .agents/             # AI dev resources (rules, prompts, agent memory)
├── CMakeLists.txt       # Root CMake build (762 lines)
├── Cargo.toml           # Rust workspace root
└── VERSION.txt          # Single source of truth (0.1.2)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Rust changes | `crates/gkit-media/src/` | Only active crate |
| C FFI changes | `apis/c/gkit-media/src/lib.rs` | 1,168-line FFI binding |
| C++ wrapper changes | `apis/cpp/gkit-media/*.hpp` | RAII headers on C handles |
| Build system changes | `CMakeLists.txt` + `cmake/GKitCargoHelpers.cmake` | Corrosion + FFI target setup |
| AI coding rules | `.agents/rules/common/` + `.agents/rules/rust/` | Layered: specific overrides general |
| Test infrastructure | See AGENTS.md notes; only gkit-media has tests | C: Unity, C++: GTest, Rust: `#[test]` |
| CI | N/A | No CI configured yet |
| Version bump | `VERSION.txt` → re-run CMake | Auto-syncs to Cargo.toml |

## BUILD

```bash
# Rust-only (fast dev loop)
cargo build -p gkit-media --features backend-native-webrtc-rs
cargo test -p gkit-media --features backend-native-webrtc-rs

# Full build (CMake + Corrosion)
cmake -B build-auto -S . -DGKIT_BUILD_TESTS=ON
cmake --build build-auto
ctest --test-dir build-auto --output-on-failure
```

**Key CMake options**: `GKIT_BUILD_TESTS`, `GKIT_BUILD_API_C`, `GKIT_BUILD_API_CPP`, `GKIT_BUILD_API_PYTHON`, `GKIT_BUILD_API_*`
**WebRTC backend**: `GKIT_FEATURE_MEDIA_WEBRTC_BACKEND` ∈ {`webrtc-rs`, `google`, `wasm`}
**First build**: Corrosion 0.6.1 extracted from `3rdparty/corrosion-0.6.1.tar.gz`, built at configure time (~minutes). vcpkg clones and bootstraps if not present.

## API BINDINGS

7 language bindings in `apis/`. Each wraps exactly 2 crates (`gkit-core` + `gkit-media`). Only C and C++ are functional; all others are stubs.

| Lang | Dir | FFI Tech | Status | Notes |
|------|-----|----------|--------|-------|
| C | `apis/c/` | extern "C" + cbindgen 0.29 | **Working** | RTC, VideoFrame, SCTP, 7 callback types |
| C++ | `apis/cpp/` | Headers on C FFI | **Working** | RAII wrappers, GTest suite, 3 examples |
| Python | `apis/python/` | PyO3 0.24 + maturin | Stub | `hello()` only |
| Node | `apis/node/` | napi-rs 2 | Stub | `hello()` only |
| WASM | `apis/wasm/` | wasm-bindgen 0.2 | Stub | `hello()` only |
| Flutter | `apis/flutter/` | flutter_rust_bridge 2 | Stub | `hello()` only |
| C# | `apis/csharp/` | csbindgen 1 | Broken | No build.rs, lib.rs is a comment |

Enable via `GKIT_BUILD_API_<LANG>` CMake options. Build helpers: `cmake/GKitCargoHelpers.cmake`.

## CONVENTIONS

- **C FFI naming**: `gkit_<crate>_<subsystem>_<resource>_<verb>[_<qualifier>]`
- **Commit**: `<feat/fix/refactor/docs/test/chore>: description`
- **Version bump**: Edit `VERSION.txt` → re-run CMake → auto-syncs to `Cargo.toml`
- **New module**: Add to `Cargo.toml` members → add to CMake `_gkit_corrosion_crates` → wire tests
- **3rdparty**: `.tar.gz` archives extracted at CMake configure time with stamp-file idempotency
- **Rust**: Edition 2024, `cargo clippy -- -D warnings`, `thiserror` for libs, `anyhow` for apps
- **C/C++**: clang-format (LLVM, Allman, 120 col, 4-space), C++11 default (configurable to C++26)
- **New crate**: Must use `version.workspace = true` / `edition.workspace = true` in Cargo.toml
- **Corrosion**: Pinned to 0.6.1; Rust crate names (hyphens) → CMake targets (underscores)
- **AI rules**: Layered `.agents/rules/` — language-specific overrides common rules

## ANTI-PATTERNS

### Rust
- **NEVER `unwrap()` in production** — use `?` with `thiserror`/`anyhow`
- **Every `unsafe` block must have `// SAFETY:` comment** explaining invariants
- **Never clone to satisfy borrow checker** — understand root cause first
- **Preferences**: `&str` over `String`, `&[T]` over `Vec<T>`, iterators over loops
- **Exhaustive match**: No wildcard `_` for business-critical enums

### C++
- **RAII everywhere** — no raw `new`/`delete`, use smart pointers
- **Never**: `malloc`/`free`, C-style arrays, `strcpy`/`strcat`/`sprintf`
- **Always**: `std::array`/`std::vector`, `std::string`, initialize variables
- **Run**: clang-format, clang-tidy, cppcheck before commits

### General
- **NEVER hardcode secrets** — use environment variables
- **Always validate at system boundaries** — fail fast
- **Always handle errors** — never silently swallow
- **Immutability**: create new objects, never mutate existing ones

## COMMANDS

```bash
# Rust development
cargo build -p gkit-media                     # Build active crate
cargo test -p gkit-media -- --nocapture       # Run tests with output
cargo clippy -- -D warnings                   # Lint (must pass)
cargo fmt                                     # Format
cargo audit && cargo deny check               # Security audit

# Full build
cmake -B build-auto -S . -DGKIT_BUILD_TESTS=ON
cmake --build build-auto
ctest --test-dir build-auto --output-on-failure

# GKit-specific
gkit-vcpkg init                               # Initialize vcpkg
gkit-vcpkg install <package>                  # Install C/C++ dependency
```

## NOTES

- **Only `gkit-media` has real code** — 7 of 8 crates are 3-line stubs
- **No CI** — project is pre-CI; add `.github/workflows/` when ready
- **Corrosion build is slow on first run** — ~minutes for full CMake configure
- **WebRTC backends are mutually exclusive** — select one at CMake configure time
- **cbindgen headers are generated at build time** — not committed, in `apis/c/*/generated/`
- **vcpkg is not a submodule** — cloned fresh if not present
- **LICENSE**: Apache 2.0 for self-owned code; third-party code must be replaced/removed for commercial use
- **AI rules**: See `.agents/rules/README.md` — `common/` for universals, `rust/`/`cpp/`/etc. for language specifics
