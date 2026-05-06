# GenericKit Rust-API CMake 集成约束

## 目录结构规范

每个 API 绑定层（c / cpp / python / wasm / node / csharp / flutter）按模块组织子目录，结构与 `apis/c/` 保持一致：

```
apis/<api>/
  CMakeLists.txt              # top-level, add_subdirectory per module
  <module>/
    CMakeLists.txt            # per-module target definition
    Cargo.toml                # Rust crate manifest (if applicable)
    src/                      # Rust source (if applicable)
    ...
```

## Cargo Workspace 成员

- 所有 API 绑定 crate 均需加入 `Cargo.toml` 的 `[workspace].members`
- `default-members` 应排除非默认构建的 crate（如 Python bindings 需 maturin）
- workspace dependency 统一在 `[workspace.dependencies]` 中声明，各 crate 通过 `xxx.workspace = true` 引用
- 所有 crate 的 `version` 和 `edition` 必须通过 `version.workspace = true` / `edition.workspace = true` 从 `[workspace.package]` 继承，不得硬编码

### 当前 workspace 结构

```toml
[workspace]
resolver = "3"
members = [
    "crates/*",           # 内部 Rust crate
    "apis/c/*",           # C FFI crate
    "apis/python/*",      # Python 绑定 crate
    "apis/wasm/*",        # Wasm 绑定 crate
    "apis/node/*",        # Node.js 绑定 crate
    "apis/flutter/*",     # Flutter 绑定 crate
    "apis/csharp/*",      # C# codegen crate
    "tools/gkit-vcpkg"    # vcpkg CLI 工具
]
default-members = ["crates/*", "apis/c/*"]
```

### workspace 级依赖

```toml
[workspace.dependencies]
gkit-core = { path = "crates/gkit-core" }
gkit-media = { path = "crates/gkit-media" }

cbindgen = { version = "0.29", default-features = false }
bindgen = { version = "0.72.1" }
csbindgen = "1"
pyo3 = { version = "0.24" }
wasm-bindgen = { version = "0.2" }
napi = { version = "2", features = ["napi4"] }
napi-derive = { version = "2" }
napi-build = { version = "2" }
flutter_rust_bridge = "2"
webrtc = "0.11"
```

## 版本号管理

统一版本号由 `VERSION.txt` 维护，CMake 和 Cargo 均从此文件读取，确保版本一致。

### VERSION.txt 格式

```
MAJOR.MINOR.PATCH
```

示例：`0.1.0`

### CMake 侧

`CMakeLists.txt` 在 `project()` 之前读取 `VERSION.txt`，解析为 `GKIT_VERSION_MAJOR` / `GKIT_VERSION_MINOR` / `GKIT_VERSION_PATCH` / `GKIT_VERSION_TWEAK`（若无 TWEAK 则设为 0）。配置阶段自动将版本同步到 `Cargo.toml` 的 `[workspace.package].version`：

```cmake
file(STRINGS "${CMAKE_SOURCE_DIR}/VERSION.txt" _gkit_version_raw LIMIT_COUNT 1 LENGTH_MINIMUM 1)
# ... regex parse into MAJOR/MINOR/PATCH/TWEAK ...
project(GenericKit VERSION ${GKIT_VERSION_MAJOR}.${GKIT_VERSION_MINOR}.${GKIT_VERSION_PATCH}.${GKIT_VERSION_TWEAK} ...)

# Sync to Cargo.toml
file(READ "${CMAKE_SOURCE_DIR}/Cargo.toml" _gkit_cargo_toml)
string(REGEX REPLACE "(\\[workspace\\.package\\]\nversion[ \t]*=[ \t]*\")[^\"]*"
    "\\1${GKIT_VERSION_STR}" _gkit_cargo_toml_new "${_gkit_cargo_toml}")
if(NOT "${_gkit_cargo_toml_new}" STREQUAL "${_gkit_cargo_toml}")
    file(WRITE "${CMAKE_SOURCE_DIR}/Cargo.toml" "${_gkit_cargo_toml_new}")
endif()
```

### Cargo 侧

根 `Cargo.toml` 中声明 `[workspace.package]`：

```toml
[workspace.package]
version = "0.1.0"
edition = "2024"
```

所有 crate 继承：

```toml
[package]
name = "gkit-core"
version.workspace = true
edition.workspace = true
```

### 更新流程

1. 修改 `VERSION.txt`
2. 重新运行 CMake 配置 → 自动更新 `Cargo.toml` `[workspace.package].version`
3. 所有 crate 通过 workspace 继承自动生效

## CMake Target 命名

```
<module>_<api>                  # 聚合 target（默认 shared）
<module>_<api>_static           # 静态链接 variant
<module>_<api>_shared           # 动态链接 variant
```

示例：`gkit_core_c`, `gkit_core_c_static`, `gkit_core_cpp`, `gkit_core_py_build` 等。

## C FFI 函数命名规范

`extern "C"` 函数遵循 `gkit_<crate>_<subsystem>_<resource>_<verb>` 模式：

```
gkit_<crate>_[<subsystem>]_<resource>_<verb>[_<qualifier>]
```

| 位置 | 说明 | 示例 |
|------|------|------|
| `gkit` | 项目前缀 | — |
| `<crate>` | 所属 crate（core / media / network 等） | `media` |
| `<subsystem>` | 子系统标识（可选，用于大 crate 内部区分） | `rtc` = WebRTC |
| `<resource>` | 操作对象 | `peer_connection`, `data_channel` |
| `<verb>` | 操作 | `create`, `destroy`, `send`, `close` |

示例：
| 函数 | 解析 |
|------|------|
| `gkit_media_hello` | crate=media, 无子系统 |
| `gkit_media_rtc_create_peer_connection` | crate=media, subsystem=rtc, resource=peer_connection, verb=create |
| `gkit_media_rtc_data_channel_send_text` | crate=media, subsystem=rtc, resource=data_channel, verb=send, qualifier=text |
| `gkit_media_rtc_free_string` | crate=media, subsystem=rtc, resource=(utility) |

> **原则**：`peer_connection`、`data_channel` 等是 WebRTC 专属名词，天然消歧。`rtc` 前缀仅在需要与同 crate 内其他子系统区分时使用（参考 libdatachannel 的 `rtc` 前缀）。
> **类型命名**：所有 C 类型（struct typedef、callback typedef）使用蛇形命名 + `_t` 后缀，如 `gkit_media_rtc_sctp_settings_t`、`gkit_media_rtc_state_callback_t`。

## FOLDER 层级（IDE 项目视图）

每个 Rust crate 及其所有关联 target 必须设置 FOLDER 属性，保持 IDE 整洁。

### 根 CMakeLists.txt 中的统一注册

使用 `corrosion_import_crate` 的 `CRATES` 参数按需导入 workspace 成员，与 `GKIT_BUILD_API_*` 选项联动：

```cmake
set(_gkit_corrosion_crates gkit-core gkit-media gkit-native gkit-crash gkit-network gkit-service)
if(GKIT_BUILD_API_C)
    list(APPEND _gkit_corrosion_crates gkit-core-c gkit-media-c)
endif()
if(GKIT_BUILD_API_PYTHON)
    list(APPEND _gkit_corrosion_crates gkit-core-py gkit-media-py)
endif()
if(GKIT_BUILD_API_WASM)
    list(APPEND _gkit_corrosion_crates gkit-core-wasm gkit-media-wasm)
endif()
if(GKIT_BUILD_API_NODE)
    list(APPEND _gkit_corrosion_crates gkit-core-node gkit-media-node)
endif()
if(GKIT_BUILD_API_FLUTTER)
    list(APPEND _gkit_corrosion_crates gkit-core-flutter gkit-media-flutter)
endif()
corrosion_import_crate(MANIFEST_PATH Cargo.toml CRATES ${_gkit_corrosion_crates})
```

> **注意**：C# codegen crate 不通过腐蚀导入（仅 build.rs codegen），因此不在 `_gkit_corrosion_crates` 中。

`gkit_cargo_set_folder` 与导入条件同步：

```cmake
# 内部 crate（始终导入）
gkit_cargo_set_folder("gkit-core"    "gkit_core")
gkit_cargo_set_folder("gkit-media"   "gkit_media")
gkit_cargo_set_folder("gkit-native"  "gkit_native")
gkit_cargo_set_folder("gkit-crash"   "gkit_crash")
gkit_cargo_set_folder("gkit-network" "gkit_network")
gkit_cargo_set_folder("gkit-service" "gkit_service")

# C FFI crate
if(GKIT_BUILD_API_C)
    gkit_cargo_set_folder("gkit-core-c"  "gkit_core/c")
    gkit_cargo_set_folder("gkit-media-c" "gkit_media/apis/c")
endif()

# Python binding crate
if(GKIT_BUILD_API_PYTHON)
    gkit_cargo_set_folder("gkit-core-py"  "gkit_core/python")
    gkit_cargo_set_folder("gkit-media-py" "gkit_media/python")
endif()

# Wasm binding crate
if(GKIT_BUILD_API_WASM)
    gkit_cargo_set_folder("gkit-core-wasm"  "gkit_core/wasm")
    gkit_cargo_set_folder("gkit-media-wasm" "gkit_media/wasm")
endif()

# Node.js binding crate
if(GKIT_BUILD_API_NODE)
    gkit_cargo_set_folder("gkit-core-node"  "gkit_core/node")
    gkit_cargo_set_folder("gkit-media-node" "gkit_media/node")
endif()

# Flutter binding crate
if(GKIT_BUILD_API_FLUTTER)
    gkit_cargo_set_folder("gkit-core-flutter"  "gkit_core/flutter")
    gkit_cargo_set_folder("gkit-media-flutter" "gkit_media/flutter")
endif()
```

> **关键原则**：`gkit_cargo_set_folder` 调用必须与 `corrosion_import_crate` 中的 `CRATES` 列表匹配。某个 crate 未被导入时调用 `gkit_cargo_set_folder` 无害（内部 `if(TARGET ...)` 守卫），但仍应与导入条件保持一致以避免混淆。

### 包装 target 的 FOLDER

由 `gkit_cargo_setup_ffi_target` / `gkit_cargo_setup_ffi_cpp_target` 的 `FOLDER` 参数控制。
当前约定的 FOLDER 值：

| target | FOLDER |
|--------|--------|
| `gkit_core_c` | `gkit_core/c` |
| `gkit_media_c` | `gkit_media/apis/c` |
| `gkit_media_cpp` | `gkit_media/apis/cpp` |

### 自定义 target 的 FOLDER

手动 `set_target_properties(... PROPERTIES FOLDER "...")`：

| target | FOLDER |
|--------|--------|
| C 测试 target | `gkit_media/apis/c/tests` |
| C++ 测试 target | `gkit_media/apis/cpp/tests` |
| Python target | `gkit_media/python` |
| Wasm target | `gkit_media/wasm` |
| Node target | `gkit_media/node` |
| C# target | `gkit_media/csharp` |
| Flutter target | `gkit_media/flutter` |
| 示例 target | `gkit_media/examples` |
| 工具 target | `gkit_tools` |

### 预期 FOLDER 树

```
gkit_core/
  cargo-build_gkit_core, _cargo-build_gkit_core, cargo-clean_gkit_core, ...
  gkit_core, gkit_core-static, gkit_core-shared
  c/
    (同上，crate: gkit-core-c)
    gkit_core_c, gkit_core_c_static, gkit_core_c_shared
  cpp/
    gkit_core_cpp, gkit_core_cpp_static, gkit_core_cpp_shared
  python/
    (cargo-build_gkit_core_py, ...)
    gkit_core_py_build, gkit_core_py_develop
  wasm/
    (cargo-build_gkit_core_wasm, ...)
    gkit_core_wasm_build_bundler, gkit_core_wasm_build_web, gkit_core_wasm_build_nodejs
  node/
    (cargo-build_gkit_core_node, ...)
    gkit_core_node_build
  csharp/
    gkit_core_csharp
  flutter/
    (cargo-build_gkit_core_flutter, ...)
    gkit_core_flutter_bindings

gkit_media/
  (cargo-build 和库 variant)
  apis/
    c/
      (gkit-media-c 腐蚀 target)
      gkit_media_c, gkit_media_c_static, gkit_media_c_shared
      tests/
        gkit_media_c_test_basic, gkit_media_c_test_sdp, gkit_media_c_test_dc,
        gkit_media_c_test_errors, gkit_media_c_test_video_frame
    cpp/
      gkit_media_cpp, gkit_media_cpp_static, gkit_media_cpp_shared
      tests/
        gkit_media_cpp_test_video_frame
  python/
    gkit_media_py_build, gkit_media_py_develop
  wasm/
    gkit_media_wasm_build_bundler, gkit_media_wasm_build_web, gkit_media_wasm_build_nodejs
  node/
    gkit_media_node_build
  csharp/
    gkit_media_csharp
  flutter/
    gkit_media_flutter_bindings
  examples/
    gkit-media-viewer, gkit-media-viewer_build, gkit-media-viewer_run, gkit_examples_assets

gkit_native/, gkit_crash/, gkit_network/, gkit_service/
gkit_tools/
  gkit-vcpkg, gkit-vcpkg_build, gkit-vcpkg_run
```

## CMake 辅助函数

| 函数 | 文件 | 用途 |
|------|------|------|
| `gkit_find_package` | `GKitFindPackageHelpers.cmake` | 增强 find_package（PROVIDED_TARGETS 校验 + target 全局化） |
| `gkit_cargo_set_folder` | `GKitCargoHelpers.cmake` | 腐蚀 target FOLDER 设置 |
| `gkit_cargo_setup_ffi_target` | `GKitCargoHelpers.cmake` | 创建 C FFI 包装 target（static+shared+aggregate） |
| `gkit_cargo_setup_ffi_cpp_target` | `GKitCargoHelpers.cmake` | 创建 C++ 包装 target（static+shared+aggregate） |
| `gkit_cargo_install_config` | `GKitCargoHelpers.cmake` | 安装 CMake config + pkgconfig 文件 |
| `gkit_cargo_cbindgen_dir` | `GKitCargoHelpers.cmake` | 获取 cbindgen 生成头文件目录 |
| `gkit_cargo_cbindgen_header` | `GKitCargoHelpers.cmake` | 获取 cbindgen 生成头文件路径 |
| `gkit_option` | `GKitOptionHelpers.cmake` | 增强 option（DEPENDS, SET, OR_CONDITION, VERIFY） |
| `gkit_install` | `GKitInstallHelpers.cmake` | 增强 install |
| `gkit_configure_process_path` | `GKitInstallHelpers.cmake` | 配置安装路径 |
| `gkit_stamp_file_info` | `GKitCMakeHelpers.cmake` | 声明 stamp 文件路径变量 |
| `gkit_fetch_3rdparty` | `GKitCMakeHelpers.cmake` | 提取 3rdparty 压缩包（.tar.gz/.tar.xz/.tar.bz2/.7z/.zip） |
| `gkit_reset_dir` | `GKitCMakeHelpers.cmake` | 删除并重建目录 |
| `gkit_make_stamp_file` | `GKitCMakeHelpers.cmake` | 创建时间戳 stamp 文件 |
| `gkit_parse_all_arguments` | `GKitCMakeHelpers.cmake` | 严格版 cmake_parse_arguments（未知参数报错） |
| `gkit_vcpkg_install` | `InstallVcpkg.cmake` | 克隆 + 引导 vcpkg（两个副本：regular + tools） |
| `gkit_vcpkg_install_package` | `InstallVcpkg.cmake` | 安装 + 导出 vcpkg 包（含 COMPONENTS 特征拼接） |

## 3rdparty 依赖模式

### 模式 A：3rdparty 压缩包（本地 tarball）

用于 GTest、Benchmark、Unity、Nuklear 等无需 vcpkg 的依赖。

```
3rdparty/<name>-<version>.tar.gz   →  cmake/FindWrap<Name>.cmake
```

FindWrap cmake 模板：

```cmake
if(TARGET GKitWrapXXX::WrapXXX)
    return()
endif()

set(GKitWrapXXX_NAME "xxx-1.2.3")
set(GKitWrapXXX_PKG_NAME "${GKitWrapXXX_NAME}.tar.gz")
set(GKitWrapXXX_DIR_NAME "${GKitWrapXXX_NAME}-${GKIT_LOWER_BUILD_TYPE}")
set(GKitWrapXXX_URL_PATH "${PROJECT_SOURCE_DIR}/3rdparty/${GKitWrapXXX_PKG_NAME}")
set(GKitWrapXXX_ROOT_DIR "${PROJECT_BINARY_DIR}/3rdparty/${GKitWrapXXX_DIR_NAME}")
set(GKitWrapXXX_BUILD_DIR "${GKitWrapXXX_ROOT_DIR}/build")
set(GKitWrapXXX_SOURCE_DIR "${GKitWrapXXX_ROOT_DIR}/source")
set(GKitWrapXXX_INSTALL_DIR "${GKitWrapXXX_ROOT_DIR}/install")

gkit_stamp_file_info(GKitWrapXXX OUTPUT_DIR "${GKitWrapXXX_ROOT_DIR}")
gkit_fetch_3rdparty(GKitWrapXXX URL "${GKitWrapXXX_URL_PATH}" OUTPUT_NAME "${GKitWrapXXX_DIR_NAME}")
if(NOT EXISTS "${GKitWrapXXX_STAMP_FILE_PATH}")
    gkit_reset_dir(${GKitWrapXXX_BUILD_DIR})

    # configure + build + install via execute_process
    execute_process(
        COMMAND ${CMAKE_COMMAND} -G ${CMAKE_GENERATOR} ... -DCMAKE_INSTALL_PREFIX=${GKitWrapXXX_INSTALL_DIR} ${GKitWrapXXX_SOURCE_DIR}
        WORKING_DIRECTORY "${GKitWrapXXX_BUILD_DIR}")
    execute_process(
        COMMAND ${CMAKE_COMMAND} --build ./ --parallel ${GKIT_NUMBER_OF_ASYNC_JOBS} --config ${CMAKE_BUILD_TYPE} --target install
        WORKING_DIRECTORY "${GKitWrapXXX_BUILD_DIR}")
    execute_process(
        COMMAND ${CMAKE_COMMAND} --install ./
        WORKING_DIRECTORY "${GKitWrapXXX_BUILD_DIR}")

    gkit_make_stamp_file("${GKitWrapXXX_STAMP_FILE_PATH}")
endif()

add_library(GKitWrapXXX::WrapXXX INTERFACE IMPORTED)
gkit_find_package(XXX PATHS ${GKitWrapXXX_INSTALL_DIR} NO_DEFAULT_PATH REQUIRED
    PROVIDED_TARGETS XXX::xxx)
target_link_libraries(GKitWrapXXX::WrapXXX INTERFACE XXX::xxx)
```

Unity 特殊处理：使用 `MATCHES 0` 替代 `EQUAL 0` 进行结果比较（兼容 Unity v2.6.1 的 CMakeLists.txt 风格）。

Nuklear 特殊处理：header-only 库，直接复制 `nuklear.h` 到 install/include，无需编译。

### 模式 B：vcpkg 包

用于 SDL3、ImGui 等由 vcpkg 管理的依赖。

```
cmake/FindWrap<Name>.cmake  → 调用 gkit_vcpkg_install_package()
```

```cmake
if(TARGET GKitWrapSDL3::WrapSDL3)
    return()
endif()

include(InstallVcpkg)
gkit_vcpkg_install_package(sdl3
    NOT_IMPORT
    TARGET GKitWrapSDL3::WrapSDL3
    PREFIX GKitWrapSDL3)

# 手动 find_package + target_link_libraries
set(CMAKE_PREFIX_PATH_BACKUP ${CMAKE_PREFIX_PATH})
set(CMAKE_PREFIX_PATH ${GKitWrapSDL3_INSTALL_DIR})
find_package(SDL3 PATHS ${GKitWrapSDL3_INSTALL_DIR} NO_DEFAULT_PATH REQUIRED)
target_link_libraries(GKitWrapSDL3::WrapSDL3 INTERFACE SDL3::SDL3)
set(CMAKE_PREFIX_PATH ${CMAKE_PREFIX_PATH_BACKUP})
```

核心函数 `gkit_vcpkg_install_package` 的参数：

| 参数 | 说明 |
|------|------|
| `TARGET` | 创建的 INTERFACE IMPORTED target 名 |
| `PREFIX` | 变量前缀（自动生成 `_{INSTALL_DIR}` 等变量） |
| `COMPONENTS` | vcpkg feature 列表，拼接为 `[c1,c2,...]` 后缀 |
| `NOT_IMPORT` | 跳过 vcpkg Config.cmake 的 find_package（需手动 link） |
| `TOOLS` | 使用 vcpkg-tools 副本而非主 vcpkg |
| `OUTPUT_DIR` | 导出目录（缺省 `build/3rdparty/vcpkg/`） |
| `QUIET` | 安装失败时 WARNING 而非 FATAL_ERROR |

### vcpkg 导出路径（关键修复）

`InstallVcpkg.cmake` 第 213-214 行：`--output=${arg_PACK_NAME}` 和 `--output-dir=${arg_OUTPUT_DIR}` **不能**有额外的 `""` 双引号。CMake `${}` 展开已经处理引号；加 `""` 会让 vcpkg 把引号当作路径字面量，导致路径拼接错误。

## DLL 导出宏体系

### 两层宏结构

所有模块共享 `apis/c/gkit-core/gkit_core_api.h` 中定义的编译器可见性宏，各模块再定义自己的上层宏。

**第一层**：`gkit_core_api.h` — 编译器级的导出/导入/隐藏声明

```c
// GCC/Clang
#define GKIT_DECLARE_EXPORT __attribute__((visibility("default")))
#define GKIT_DECLARE_IMPORT __attribute__((visibility("default")))
#define GKIT_DECLARE_HIDDEN __attribute__((visibility("hidden")))

// MSVC
#define GKIT_DECLARE_EXPORT __declspec(dllexport)
#define GKIT_DECLARE_IMPORT __declspec(dllimport)
```

**第二层**：每个模块自己的 API 头文件（如 `gkit_media_api.h`）— 模块级宏

```c
#include <gkit_core_api.h>

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

- 静态链接时 `GKIT_BUILD_SHARED` 未定义，所有 `*_API` 宏展开为空
- `GKIT_MEDIA_API` 通过 cbindgen 的 `[fn] prefix` 自动注入到每个函数声明前
- 新增模块时：
  1. 在 `apis/c/gkit-core/gkit_core_api.h` 追加 `GKIT_DECLARE_*` 宏（如已存在则复用）
  2. 创建 `apis/c/<module>/gkit_<module>_api.h` 定义模块级宏
  3. 在 cbindgen.toml 中配置 `header = '#include "gkit_<module>_api.h"'`
  4. 在 `apis/c/CMakeLists.txt` 中 `gkit_install` 该 .h 文件

## C 头文件生成规范（cbindgen）

### 命名风格

生成头文件中的所有类型使用**蛇形命名** + 功能前缀：

```
gkit_<module>_<subsystem>_<name>_t     # typedef 类型
```

| 旧风格 | 新风格 |
|--------|--------|
| `RtcSctpSettings` | `gkit_media_rtc_sctp_settings_t` |
| `PcStateCallback` | `gkit_media_rtc_state_callback_t` |

### cbindgen.toml 关键配置

```toml
[fn]
prefix = "GKIT_MEDIA_API "    # 每个函数声明前注入 DLL 导出宏

header = '#include "gkit_media_api.h"'  # 生成头文件顶部 include

[export.rename]
"RustTypeName" = "c_name_t"   # 类型重命名为蛇形命名
```

### cbindgen build.rs 调用

```rust
let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
cbindgen::Builder::new()
    .with_crate(crate_dir)
    .with_namespace("gkit_media")
    .with_style(cbindgen::Style::Tag)
    .generate()
    .expect("cbindgen failed")
    .write_to_file("generated/gkit_media.h");
```

### 约束

- **cbindgen 0.29 限制**：同 crate 多次调用 `generate()` 时 `[export] include/exclude` 过滤器不生效，目前单头文件方案
- 生成头文件统一输出到 `apis/<crate>/generated/`
- 手动头文件（如 `gkit_media_api.h`）放在 `apis/<crate>/` 目录
- cbindgen workspace 依赖固定版本 0.29（已知限制，待升级 ≥0.30 拆分多文件头）

## WebRTC 后端编译开关（CORROSION_FEATURES）

使用腐蚀的 `CORROSION_FEATURES` target 属性注入 Cargo feature flag：

```cmake
if(GKIT_FEATURE_MEDIA_WEBRTC_BACKEND STREQUAL "webrtc-rs")
    set_target_properties(gkit_media PROPERTIES CORROSION_FEATURES "backend-native-webrtc-rs")
elseif(GKIT_FEATURE_MEDIA_WEBRTC_BACKEND STREQUAL "google")
    set_target_properties(gkit_media PROPERTIES CORROSION_FEATURES "backend-native-google")
elseif(GKIT_FEATURE_MEDIA_WEBRTC_BACKEND STREQUAL "wasm")
    set_target_properties(gkit_media PROPERTIES CORROSION_FEATURES "backend-wasm")
endif()
```

GKIT_FEATURE_MEDIA_WEBRTC_BACKEND 是一个 CMake cache string，可选值 `webrtc-rs` / `google` / `wasm`：
- wasm32/EMSCRIPTEN 平台强制 `wasm`，不可选择其他
- 非 wasm 平台默认 `webrtc-rs`

### Cargo.toml feature 对应

```toml
[features]
default = ["backend-native"]
backend-native = []
backend-native-webrtc-rs = ["backend-native"]
backend-native-google = ["backend-native"]
backend-wasm = []
```

## 内部 Rust crate 列表

| crate | crate-type | 路径 | 说明 |
|-------|-----------|------|------|
| `gkit-core` | lib, staticlib, cdylib | `crates/gkit-core/` | 核心库 |
| `gkit-media` | lib, staticlib, cdylib | `crates/gkit-media/` | 媒体/WebRTC（最开发完善） |
| `gkit-native` | lib | `crates/gkit-native/` | 原生平台集成 |
| `gkit-crash` | lib | `crates/gkit-crash/` | 崩溃处理 |
| `gkit-network` | lib, staticlib, cdylib | `crates/gkit-network/` | 网络库 |
| `gkit-service` | lib, staticlib, cdylib | `crates/gkit-service/` | 服务库 |
| `gkit-profiling` | lib, staticlib, cdylib | `crates/gkit-profiling/` | 性能分析（stub） |
| `gkit-graphics` | lib, staticlib, cdylib | `crates/gkit-graphics/` | 图形库（stub） |

> `gkit-profiling` 和 `gkit-graphics` 当前为 stub crate（仅 `hello()` 函数），尚未加入腐蚀导入列表和 cargo redirect 列表。集成时将按相同模式添加到 `_gkit_corrosion_crates`、`_gkit_redirect_crates` 和 FOLDER 注册。

## Python 绑定特殊规则

### 构建工具

- Python 绑定使用 `maturin build` / `maturin develop`
- 顶层 `apis/python/CMakeLists.txt` 必须以 `find_program(GKIT_MATURIN_EXECUTABLE maturin)` 查找 maturin，**不可设为 `REQUIRED`**；未找到时 `message(WARNING ...)` 并 `return()`，避免阻断整个 CMake 配置
- 所有 maturin 命令使用 `${GKIT_MATURIN_EXECUTABLE}` 而非裸 `maturin`

### CMake Target 命名（Python）

```
<module>_py_build            # maturin build --release --out <dir> → .whl（ALL 目标）
<module>_py_develop          # maturin develop → 安装到当前 Python 环境
```

> **注意**：腐蚀为 Python crate 创建的同名 IMPORTED library target（如 `gkit_core_py`）不可被自定义 target 覆盖。因此不再使用 `<module>_py` 聚合 target，`_build` target 直接设为 `ALL`。

### Custom Target 要求

- `add_custom_target` 必须设置 `USES_TERMINAL` 以显示 maturin 实时输出
- 所有 Python target 的 FOLDER 归入 `<module>/python/`，由 `set_target_properties` 手动设置

### Cargo Workspace

- Python crates 加入 `members` 但不加入 `default-members`（需 maturin 构建）
- `cargo check -p <crate>` 应可通过（compile-only），完整构建需 maturin

## WebAssembly 绑定特殊规则

### 构建工具

- Wasm 绑定使用 `wasm-pack build`
- 顶层 `apis/wasm/CMakeLists.txt` 自动检测并安装 `wasm-pack`（通过 `cargo install wasm-pack`）；`wasm32-unknown-unknown` target 仅检测不安装，缺失时 `message(WARNING ...)` 提示手动安装
- 所有 wasm-pack 命令使用 `${GKIT_WASM_PACK_EXECUTABLE}` 而非裸 `wasm-pack`

### CMake Target 命名（Wasm）

```
<module>_wasm_build_bundler    # wasm-pack build --release --target bundler（ALL）
<module>_wasm_build_web        # wasm-pack build --release --target web（ALL）
<module>_wasm_build_nodejs     # wasm-pack build --release --target nodejs（ALL）
```

### Custom Target 要求

- `add_custom_target` 必须设置 `USES_TERMINAL` 以显示 wasm-pack 实时输出
- 所有 Wasm target 的 FOLDER 归入 `<module>/wasm/`，由 `set_target_properties` 手动设置
- 输出目录结构：`build/wasm/{bundler,web,nodejs}/`，每种 target 输出到各自子目录

### Cargo Workspace

- Wasm crates 加入 `members` 但不加入 `default-members`（需 wasm-pack 构建）
- `cargo check -p <crate>` 应可通过（compile-only），完整构建需 `wasm-pack build`

## Node.js 绑定特殊规则（napi-rs）

### 构建方式

- Node.js 绑定使用 napi-rs，无需外部 CLI — 由腐蚀（corrosion）直接调用 `cargo build` 编译 cdylib
- CMake 自定义 target 将腐蚀产物复制为 `.node` 文件
- 顶层 `apis/node/CMakeLists.txt` 仅 `add_subdirectory`，无需查找外部工具

### CMake Target 命名（Node.js）

```
<module>_node_build          # 复制 .dylib/.so/.dll → build/node/<module>.node（ALL）
```

### Custom Target 要求

- 使用 `$<TARGET_FILE:<crate>-shared>` 生成器表达式定位腐蚀产物
- `DEPENDS cargo-build_<crate>` 确保先构建 Rust 再复制
- 输出目录：`${GKIT_BUILD_DIR}/node/`

### Cargo Workspace

- Node crates 加入 `members` 但不加入 `default-members`
- crate-type 为 `["cdylib"]`，napi-build 作为 build-dependency
- `cargo check -p <crate>` 应可通过

## C# 绑定特殊规则（csbindgen）

### 构建方式

- C# 绑定由独立的 codegen crate（`apis/csharp/<module>/`）的 `build.rs` 调用 csbindgen 生成 `.cs` P/Invoke 源文件
- 代码生成在 CMake 配置阶段通过 `execute_process` + 环境变量 `CSHARP_OUT_DIR` 执行：`cargo build -p gkit-media-csharp-codegen`
- 原生动态库由 C FFI crate 编译，自动复制到 `${GKIT_BUILD_DIR}/csharp/lib/<rid>/`（按平台 RID 组织）
- `add_custom_target(... ALL)` 负责库复制步骤；codegen crate 不通过腐蚀导入（仅 build.rs 代码生成）

### codegen crate 命名

```
gkit-<module>-csharp-codegen  # crate name 中含 "-csharp-codegen" 后缀
```

例如：`gkit-media-csharp-codegen` 在目录 `apis/csharp/gkit-media/` 中。

### 输出目录结构

```
build/csharp/
├── src/                         # 生成的 C# 源文件
│   ├── gkit_core.NativeMethods.g.cs
│   └── gkit_media.NativeMethods.g.cs
└── lib/
    └── <rid>/                   # 平台运行时标识
        ├── libgkit_core_c.dylib
        └── libgkit_media_c.dylib
```

### CMake Target 命名（C#）

```
<module>_csharp              # ALL custom target（代码生成 + 库复制）
```

### Cargo Workspace

- codegen crate 加入 `members = ["apis/csharp/*"]`，但不加入 `default-members`
- crate-type 无（纯 codegen，仅 build.rs 有副作用）
- csbindgen 作为 build-dependency，加入 `[workspace.dependencies]`

## Flutter 绑定特殊规则（flutter_rust_bridge）

### 构建方式

- Flutter 绑定使用 `flutter_rust_bridge_codegen` 进行代码生成
- 代码生成在 CMake 配置阶段执行（`execute_process`）：从 `src/api/` 读取 Rust API 函数 → 生成 `src/frb_generated.rs`（Rust 胶水代码）+ Dart 文件到 `build/flutter/<module>/dart/`
- 腐蚀编译 Flutter crate → CMake custom target 将原生动态库复制到 `build/flutter/lib/<rid>/`
- 顶层 `apis/flutter/CMakeLists.txt` 自动检测并安装 `flutter_rust_bridge_codegen` + 检测 Dart SDK + 计算平台 RID

### Dart SDK 依赖

- Flutter bindings 需要 `dart` 命令行工具（通过 `find_program(GKIT_DART_EXECUTABLE dart)` 查找）
- 未找到时 WARNING 并 return()，不阻断 CMake 配置

### 平台 RID 计算

```cmake
if(APPLE)
    if(CMAKE_SYSTEM_PROCESSOR MATCHES "arm64|aarch64")
        set(GKIT_FLUTTER_RID "osx-arm64")
    else()
        set(GKIT_FLUTTER_RID "osx-x64")
    endif()
elseif(WIN32)
    set(GKIT_FLUTTER_RID "win-x64" / "win-x86")
else()
    set(GKIT_FLUTTER_RID "linux-x64" / "linux-x86")
endif()
```

### 输出目录结构

```
build/flutter/
├── gkit-core/dart/              # FRB 生成的 Dart 文件
│   ├── frb_generated.dart
│   └── hello.dart
├── gkit-media/dart/
└── lib/<rid>/                   # 原生动态库
    ├── libgkit_core_flutter.dylib
    └── libgkit_media_flutter.dylib
```

### CMake Target 命名（Flutter）

```
<module>_flutter_bindings       # ALL custom target（代码生成 + 库复制）
```

> **注意**：腐蚀为 Flutter crate 创建的同名 IMPORTED target（如 `gkit_core_flutter`）不可被覆盖。使用 `_bindings` 后缀避免冲突。

### 代码生成命令

```cmake
execute_process(
    COMMAND "${GKIT_FRB_CODEGEN}" generate
        --rust-input "apis/flutter/gkit-media/src/api"
        --rust-root "apis/flutter/gkit-media"
        --dart-output "${_dart_out}"
        --rust-output "apis/flutter/gkit-media/src/frb_generated.rs"
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
```

### Cargo Workspace

- Flutter crates 加入 `members` 但不加入 `default-members`（需 flutter_rust_bridge_codegen）
- `cargo check -p <crate>` 应可通过（compile-only），完整绑定生成需 `flutter_rust_bridge_codegen generate`

## 输出目录规范

- **禁止 Rust 产物输出到 build 根目录**：所有腐蚀（corrosion）生成的 `.dylib` / `.so` / `.a` / `.dll` / `.lib` 必须通过 `ARCHIVE_OUTPUT_DIRECTORY` / `LIBRARY_OUTPUT_DIRECTORY` / `RUNTIME_OUTPUT_DIRECTORY` 重定向到子目录。CMake 构建后 `build/` 根目录不应出现任何 Rust 编译产物。
- **内部 crate**（`gkit-core`, `gkit-media`, `gkit-native`, `gkit-crash`, `gkit-network`, `gkit-service`）：输出到 `${GKIT_BUILD_DIR}/cargo/<crate>/`
- **Python 绑定 crate**（`gkit-core-py`, `gkit-media-py`）：输出到 `${GKIT_BUILD_DIR}/cargo/<crate>/`（腐蚀生成，非 maturin 产物）
- **Wasm 绑定 crate**（`gkit-core-wasm`, `gkit-media-wasm`）：输出到 `${GKIT_BUILD_DIR}/cargo/<crate>/`（腐蚀生成，非 wasm-pack 产物）
- **Node.js 绑定 crate**（`gkit-core-node`, `gkit-media-node`）：输出到 `${GKIT_BUILD_DIR}/cargo/<crate>/`（腐蚀生成）
- **Flutter 绑定 crate**（`gkit-core-flutter`, `gkit-media-flutter`）：输出到 `${GKIT_BUILD_DIR}/cargo/<crate>/`（腐蚀生成）
- **FFI 导出库**（C FFI crate）：输出到 `${GKIT_BUILD_DIR}/lib/`（由 `gkit_cargo_setup_ffi_target` 设置）
- **Python whl 包**：输出到 `${GKIT_BUILD_DIR}/dist/`（通过 `maturin build --out`）
- **Wasm 包**：输出到 `${GKIT_BUILD_DIR}/wasm/{bundler,web,nodejs}/`（通过 `wasm-pack build --out-dir`）
- **Node.js 绑定**：输出到 `${GKIT_BUILD_DIR}/node/`（`.node` 文件）
- **Flutter 绑定**：Dart 文件输出到 `${GKIT_BUILD_DIR}/flutter/<module>/dart/`，原生库到 `${GKIT_BUILD_DIR}/flutter/lib/<rid>/`
- **C# 绑定**：源文件输出到 `${GKIT_BUILD_DIR}/csharp/src/`，原生库到 `${GKIT_BUILD_DIR}/csharp/lib/<rid>/`
- 安装期路径：`GKIT_INSTALL_LIBDIR`, `GKIT_INSTALL_DLLDIR`, `GKIT_INSTALL_INCLUDEDIR`

## 构建与自动化测试

- CMake 构建目录统一使用 `build-auto`
- 自动化测试命令：

```bash
cmake -B build-auto -S . -DGKIT_BUILD_TESTS=ON
cmake --build build-auto
ctest --test-dir build-auto
cargo test -p gkit-media
```

### CTest 注册（根 CMakeLists.txt）

```cmake
enable_testing()
gkit_find_package(WrapUnity PROVIDED_TARGETS GKitWrapUnity::WrapUnity)
gkit_find_package(WrapGTest PROVIDED_TARGETS GKitWrapGTest::WrapGTest)
gkit_find_package(WrapBenchmark PROVIDED_TARGETS GKitWrapBenchmark::WrapBenchmark)

# Cargo test 注册为 CTest
add_test(NAME gkit_media_tests_native
    COMMAND "${GKIT_CARGO_EXECUTABLE}" test -p gkit-media --features backend-native
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
add_test(NAME gkit_media_tests_google
    COMMAND ${CMAKE_COMMAND} -E env GKIT_SKIP_WEBRTC_DOWNLOAD=true
        "${GKIT_CARGO_EXECUTABLE}" test -p gkit-media --features backend-native-google
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")
```

### 测试矩阵

| 层 | 语言 | 框架 | 数量 | FOLDER |
|----|------|------|------|--------|
| C FFI | C | Unity (`#include "gkit_media.h"`) | 5 (.c) | `gkit_media/apis/c/tests` |
| C++ FFI | C++ | GTest (`#include <gkit_media_video_frame.hpp>`) | 1 (.cpp) | `gkit_media/apis/cpp/tests` |
| Rust trait (webrtc-rs) | Rust | `#[test]` | 21 | — |
| Rust trait (google) | Rust | `#[test]` | 21 | — |

C 测试文件列表：
- `test_basic.c` — PeerConnection create/destroy 生命周期
- `test_sdp.c` — SDP offer/answer + ICE
- `test_data_channel.c` — DataChannel label/send/close
- `test_errors.c` — null handles, closed peer 错误码
- `test_video_frame.c` — VideoFrame create/destroy, scale/crop/rotate

## C/C++ FFI 测试约定

- C FFI 层（`apis/c/`）的测试必须用**纯 C 语言**编写，`#include` 对应生成的 `.h` 文件
- C++ FFI 层（`apis/cpp/`）的测试使用 **GTest** 框架
- 每个测试 `.c`/`.cpp` 文件编译为独立可执行文件
- C 测试 target 的 FOLDER 归入 `<module>/apis/c/tests`
- C++ 测试 target 的 FOLDER 归入 `<module>/apis/cpp/tests`
- 测试通过 `add_test(NAME ... COMMAND <target>)` 注册 CTest
- 测试目录：`apis/c/<module>/tests/`、`apis/cpp/<module>/tests/`
- C 测试的 cmake 模板：
  ```cmake
  gkit_find_package(WrapUnity PROVIDED_TARGETS GKitWrapUnity::WrapUnity)
  add_executable(gkit_media_c_test_basic "${_test_dir}/test_basic.c")
  target_link_libraries(gkit_media_c_test_basic gkit_media_c GKitWrapUnity::WrapUnity)
  set_target_properties(gkit_media_c_test_basic PROPERTIES FOLDER "gkit_media/apis/c/tests")
  add_test(NAME gkit_media_c_test_basic COMMAND gkit_media_c_test_basic)
  ```

## checker

每个新的 API 绑定 crate 加入后确认：
1. `Cargo.toml` 中 `members` 包含该 crate，`default-members` 按需排除
2. 根 `CMakeLists.txt` 中 `_gkit_corrosion_crates` 列表包含该 crate，且 `gkit_cargo_set_folder` 调用与导入条件同步
3. `apis/<api>/<mod>/CMakeLists.txt` 中创建对应的 CMake target
4. IDE FOLDER 树中该 crate 的 `cargo-build`, `cargo-clean`, `cargo-prebuild` 等腐蚀 target 归类正确
5. `cargo build`（默认 members）可正常编译
6. `build/` 根目录无 Rust 编译产物（`.dylib`/`.so`/`.a`/`.dll`/`.lib`），所有 crate 输出已重定向到 `build/cargo/<crate>/` 或 `build/lib/`

全新的 Rust crate 加入后确认：
1. 该 crate 的 `Cargo.toml` 使用 `version.workspace = true` 和 `edition.workspace = true`（继承自 `[workspace.package]`）
2. 根 `Cargo.toml` 的 `[workspace.dependencies]` 中包含该 crate 的 path dependency（如有引用）
3. `VERSION.txt` 版本更新后，CMake 重新配置可自动同步到根 `Cargo.toml`

## VideoFrame 模块

### 目录结构

```
crates/gkit-media/src/video/
├── mod.rs           # pub mod buffer/frame/convert/transform
├── buffer.rs        # VideoBuffer trait + I420/NV12/I422/I444/I010 缓冲区
├── frame.rs         # VideoFrame<T>, VideoRotation, FrameMetadata
├── convert.rs       # to_i420, i420_to_argb (YUV ↔ RGBA)
└── transform.rs     # i420_scale/crop/rotate (缩放/裁剪/旋转)
```

### C/C++ 绑定
- C FFI：14 个 `gkit_media_video_frame_*` 函数（创建/销毁/属性/平面数据/缩放/裁剪/旋转）
- C++：`gkit::VideoFrame` RAII 类（`gkit_media_video_frame.hpp`），move-only 语义
- 测试：C 层 Unity + C++ 层 GTest, FOLDER `gkit_media/apis/c/tests` 和 `gkit_media/apis/cpp/tests`

## CMake 配置细节

### .cmake.conf（策略设置）

```cmake
set(GKIT_MIN_NEW_POLICY_CMAKE_VERSION 3.22)
set(GKIT_MAX_NEW_POLICY_CMAKE_VERSION 3.31)
cmake_policy(SET CMP0075 NEW)  # CMAKE_REQUIRED_LIBRARIES
cmake_policy(SET CMP0083 NEW)  # check_pie_supported
cmake_policy(SET CMP0091 NEW)  # CMAKE_MSVC_RUNTIME_LIBRARY
```

### CMake 最低版本与平台

```cmake
cmake_minimum_required(VERSION 3.22...3.31)
project(GenericKit ... LANGUAGES CXX C)
```

### C++ 标准

- 默认 C++11（可通过 `CMAKE_CXX_STANDARD` 覆盖为 11/14/17/20/23/26）
- `CMAKE_CXX_STANDARD_REQUIRED ON`
- 检查 MSVC ≥16.0，GCC ≥4.7

### PIC 与 PIE

- `CMAKE_POSITION_INDEPENDENT_CODE ON`
- `check_pie_supported()` 检查链接器 PIE 支持

### 共享库开关

```cmake
gkit_option(GKIT_BUILD_SHARED_LIBS "Enable this to build as dynamically" ON SET BUILD_SHARED_LIBS)
```

### 3rdparty 引入

```cmake
include(InstallCorrosion)       # 腐蚀（Rust-CMake 桥接）
include(InstallVcpkg)           # vcpkg 包管理
include(InstallRequiredSystemLibraries)  # 系统库
```

## 工具

### gkit-vcpkg

Rust CLI 工具（`tools/gkit-vcpkg/`），基于 clap，8 个子命令：`init`, `find`, `install`, `export`, `cargo-config`, `cmake-toolchain`, `list`。

CMake target：`gkit-vcpkg_build`（`cargo build`）、`gkit-vcpkg_run`（`cargo run -- --help`）、`gkit-vcpkg`（aggregate）。

### gkit-rc（gkit-rc）

占位工具（`tools/gkit-rc/`），Crate 名 `gresc`，当前仅 hello world stub。**未加入 workspace members**。

## Rust 示例

Rust 示例统一放在 `crates/<crate>/examples/` 目录下（遵循 Cargo 约定）。

### gkit-media-viewer

`crates/gkit-media/examples/gkit-media-viewer/`：
- egui 应用：grid + single view 双标签，展示 VideoFrame 变换效果
- BMP → I420 → transform → RGBA 管线
- 加载 assets 目录，通过 CMake `gkit_examples_assets` target 复制到 `build/assets/`
- CMake target 命名：`<name>_build`（编译）、`<name>_run`（运行）、`<name>`（aggregate ALL）
- FOLDER：`gkit_media/examples`
