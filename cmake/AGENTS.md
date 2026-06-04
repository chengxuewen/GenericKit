# cmake AGENTS.md

**Parent:** [../AGENTS.md](../AGENTS.md) &mdash; root conventions, build, anti-patterns

## OVERVIEW

19 CMake modules implementing GenericKit's hybrid CMake+Cargo build system. Corrosion 0.6.1 bridges CMake and Rust. All modules use the `gkit_` prefix for functions/macros.

## STRUCTURE

```
cmake/
├── GKitCMakeHelpers.cmake      # Core utilities: fetch 3rdparty, stamp files, path operations
├── GKitCargoHelpers.cmake      # ★ KEY — Corrosion integration, FFI target setup, cbindgen
├── GKitCargoExample.cmake      # Rust example build via CMake
├── GKitOptionHelpers.cmake     # gkit_option() macro (DEPENDS, SET, VERIFY)
├── GKitPlatformHelpers.cmake   # Platform/arch/compiler detection (ported from Qt)
├── GKitInstallHelpers.cmake    # gkit_install() with export handling
├── GKitFindPackageHelpers.cmake # Enhanced find_package with target validation
├── InstallCorrosion.cmake      # Fetch/build corrosion-0.6.1 from 3rdparty/ tarball
├── InstallVcpkg.cmake          # Clone/bootstrap vcpkg, gkit_vcpkg_install_package()
├── InstallPython.cmake         # Python installation (included but unused)
├── FindWrapGTest.cmake         # GTest 1.12.1 finder (auto-extracts from 3rdparty/)
├── FindWrapUnity.cmake         # Unity 2.6.1 finder
├── FindWrapBenchmark.cmake     # Google Benchmark 1.8.4 finder
├── FindWrapSDL3.cmake          # SDL3 finder
├── FindWrapImGui.cmake         # ImGui 1.92.7 finder
├── FindWrapCImGui.cmake        # CImGui finder
├── GKitCargoConfig.cmake.in    # Template for C FFI CMake config
├── GKitCargoCppConfig.cmake.in # Template for C++ FFI CMake config
└── GKitTarget.pc.in            # Template for pkg-config .pc file
```

## WHERE TO LOOK

| Task | Location | Key Function |
|------|----------|-------------|
| Add new Rust crate to CMake | `GKitCargoHelpers.cmake` | `gkit_cargo_setup_ffi_target()` |
| Add C++ wrapper on C FFI | `GKitCargoHelpers.cmake` | `gkit_cargo_setup_ffi_cpp_target()` |
| Change cbindgen behavior | `GKitCargoHelpers.cmake` | `gkit_cargo_cbindgen_dir()`, `gkit_cargo_cbindgen_header()` |
| Add 3rdparty dependency | `GKitCMakeHelpers.cmake` | `gkit_fetch_3rdparty()` + stamp file pattern |
| Add FindWrap module | Follow `FindWrapGTest.cmake` pattern | tarball → extract → target wrapper |
| Add build option | `GKitOptionHelpers.cmake` | `gkit_option()` macro |
| Change platform detection | `GKitPlatformHelpers.cmake` | OS/arch/compiler variables |
| Add install rule | `GKitInstallHelpers.cmake` | `gkit_install()` |
| Change package finding | `GKitFindPackageHelpers.cmake` | `gkit_find_package()` |

## KEY PATTERNS

### 1. FFI Target Layering
```
Rust crate (corrosion_import_crate)
  → C FFI target (gkit_cargo_setup_ffi_target)
    → C++ wrapper target (gkit_cargo_setup_ffi_cpp_target)
      → CMake Config + pkg-config (gkit_cargo_install_config)
```

### 2. 3rdparty Tarball Pattern
```cmake
# Each dependency: .tar.gz in 3rdparty/ → extracted at configure time → stamp file
gkit_fetch_3rdparty(
    NAME GTest DIR ${3rdparty_dir} URL ${3rdparty_dir}/googletest-release-1.12.1.tar.gz
    DESTINATION ${CMAKE_BINARY_DIR}/3rdparty
    STAMP ${CMAKE_BINARY_DIR}/3rdparty/GTest-stamp.txt
)
```

### 3. Naming Convention
Custom CMake targets use hyphen style (matching Rust crate names). Corrosion-generated internal targets use underscore style:
- `gkit-media` → `gkit_media` (Corrosion auto-generated target — underscores)
- `gkit-media-c` → `gkit-media-c` (C FFI wrapper target — hyphens)
- `gkit-media-cpp` → `gkit-media-cpp` (C++ wrapper target — hyphens)
- Corrosion internal: `gkit_media_c`, `gkit_media_c-shared`, `cargo-build_gkit_media_c` (auto-generated — underscores)

### 4. Build Options (gkit_option)
```cmake
gkit_option(
    NAME GKIT_BUILD_API_PYTHON
    SET ON
    DEPENDS "maturin_FOUND"
    VERIFY "maturin command not found"
)
```

## CONVENTIONS

- **All helper functions prefixed `gkit_`** — use lowercase with underscores
- **`corrosion_import_crate` is called ONCE** — all crates in one call, not per crate
- **Corrosion target features**: set via `set_target_properties(... CORROSION_FEATURES ...)`
- **Cargo output redirection**: `build-auto/cargo/<crate>/` per crate
- **Stamp files**: `<Name>-stamp.txt` in build dir — idempotent configure
- **IDE folders**: `gkit_cargo_set_folder(crate "gkit-media/ffi")` for IDE tree
- **Debug postfixes**: macOS: `_debug`, Windows: `d`, Linux: none

## NOTES

- **Corrosion 0.6.1 is pinned** — upgrading may break FFI target macros
- **First CMake configure is slow** — Corrosion + vcpkg bootstrap (minutes)
- **In-source build is fatal** — `PROJECT_SOURCE_DIR == CMAKE_BINARY_DIR` blocked
- **VERSION.txt auto-syncs to Cargo.toml** — line 58-65 of root CMakeLists.txt
- **vcpkg has two copies**: `vcpkg/` (mutable) + `vcpkg-tools/` (clean copy for tools)
