# GenericKit Rust-API CMake 集成约束

## 目录结构规范

每个 API 绑定层（c / cpp / python）按模块组织子目录，结构与 `apis/c/` 保持一致：

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

## FOLDER 层级（IDE 项目视图）

每个 Rust crate 及其所有关联 target 必须设置 FOLDER 属性，保持 IDE 整洁：

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
corrosion_import_crate(MANIFEST_PATH Cargo.toml CRATES ${_gkit_corrosion_crates})
```

`gkit_cargo_set_folder` 与导入条件同步：

```cmake
# 内部 crate（始终导入）
gkit_cargo_set_folder("gkit-core"   "gkit_core")
gkit_cargo_set_folder("gkit-media"  "gkit_media")

# C FFI crate
if(GKIT_BUILD_API_C)
    gkit_cargo_set_folder("gkit-core-c" "gkit_core/c")
    gkit_cargo_set_folder("gkit-media-c" "gkit_media/c")
endif()

# Python binding crate
if(GKIT_BUILD_API_PYTHON)
    gkit_cargo_set_folder("gkit-core-py"  "gkit_core/python")
    gkit_cargo_set_folder("gkit-media-py" "gkit_media/python")
endif()
```

> **关键原则**：`gkit_cargo_set_folder` 调用必须与 `corrosion_import_crate` 中的 `CRATES` 列表匹配。某个 crate 未被导入时调用 `gkit_cargo_set_folder` 无害（内部 `if(TARGET ...)` 守卫），但仍应与导入条件保持一致以避免混淆。

### `gkit_cargo_set_folder` 负责设置哪些 target

该函数自动为 crate 名的以下 CMake target 设置 FOLDER：
- `cargo-build_<name>` — cargo 构建 custom target
- `_cargo-build_<name>` — 内部辅助 target
- `cargo-clean_<name>` — cargo clean custom target
- `cargo-prebuild_<name>` — 预构建 custom target
- `<name>` — 库 target
- `<name>-static` / `<name>-shared` — 库变体

### 包装 target 的 FOLDER

由 `gkit_cargo_setup_ffi_target` / `gkit_cargo_setup_ffi_cpp_target` 的 `FOLDER` 参数控制。

### 自定义 target 的 FOLDER

手动 `set_target_properties(... PROPERTIES FOLDER "...")`，如 Python 的 `gkit_core_py` 等。

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
    (cargo-build_gkit_core_py, cargo-clean_gkit_core_py, ...)
    gkit_core_py_build, gkit_core_py_develop
  wasm/
    (cargo-build_gkit_core_wasm, cargo-clean_gkit_core_wasm, ...)
    gkit_core_wasm_build_bundler, gkit_core_wasm_build_web, gkit_core_wasm_build_nodejs
  node/
    (cargo-build_gkit_core_node, cargo-clean_gkit_core_node, ...)
    gkit_core_node_build
  csharp/
    gkit_core_csharp
  flutter/
    (cargo-build_gkit_core_flutter, cargo-clean_gkit_core_flutter, ...)
    gkit_core_flutter_bindings

gkit_media/
  ...(同上结构)
```

## CMake 辅助函数使用

| 函数 | 用途 | 调用位置 |
|------|------|----------|
| `gkit_cargo_set_folder` | 锈蚀 target FOLDER 设置 | 根 CMakeLists.txt |
| `gkit_cargo_setup_ffi_target` | 创建 C FFI 包装 target（static+shared） | `apis/c/<mod>/CMakeLists.txt` |
| `gkit_cargo_setup_ffi_cpp_target` | 创建 C++ 包装 target（static+shared） | `apis/cpp/<mod>/CMakeLists.txt` |
| `gkit_cargo_install_config` | 安装 CMake config + pkgconfig 文件 | 各模块 CMakeLists.txt |

## Python 绑定特殊规则

### 构建工具

- Python 绑定使用 `maturin build` / `maturin develop`
- 顶层 `apis/python/CMakeLists.txt` 必须以 `find_program(GKIT_MATURIN_EXECUTABLE maturin)` 查找 maturin，**不可设为 `REQUIRED`**；未找到时 `message(WARNING ...)` 并 `return()`，避免阻断整个 CMake 配置
- 所有 maturin 命令使用 `${GKIT_MATURIN_EXECUTABLE}` 而非裸 `maturin`

### CMake Target 命名（Python）

```
<module>_py_build            # maturin build --release → target/wheels/*.whl（ALL 目标）
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
- CMake 配置阶段通过 `execute_process` + 环境变量 `CSHARP_OUT_DIR` 将源文件生成到 `${GKIT_BUILD_DIR}/csharp/src/`
- 原生动态库由 C FFI crate 编译，自动复制到 `${GKIT_BUILD_DIR}/csharp/lib/<rid>/`（按平台 RID 组织）
- `add_custom_target(... ALL)` 负责库复制步骤，codegen crate 不通过腐蚀导入

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
<module>_csharp              # ALL custom target，DEPENDS cargo-build_<codegen_crate>
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
- 顶层 `apis/flutter/CMakeLists.txt` 自动检测并安装 `flutter_rust_bridge_codegen` + 检测平台 RID

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

### Cargo Workspace

- Flutter crates 加入 `members` 但不加入 `default-members`（需 flutter_rust_bridge_codegen）
- `cargo check -p <crate>` 应可通过（compile-only），完整绑定生成需 `flutter_rust_bridge_codegen generate`

## 输出目录规范

- **禁止 Rust 产物输出到 build 根目录**：所有腐蚀（corrosion）生成的 `.dylib` / `.so` / `.a` / `.dll` / `.lib` 必须通过 `ARCHIVE_OUTPUT_DIRECTORY` / `LIBRARY_OUTPUT_DIRECTORY` / `RUNTIME_OUTPUT_DIRECTORY` 重定向到子目录。CMake 构建后 `build/` 根目录不应出现任何 Rust 编译产物。
- **内部 crate**（`gkit-core`, `gkit-media`, `gkit-native`, `gkit-crash`, `gkit-network`, `gkit-service`）：输出到 `${GKIT_BUILD_DIR}/cargo/<crate>/`
- **Python 绑定 crate**（`gkit-core-py`, `gkit-media-py`）：输出到 `${GKIT_BUILD_DIR}/cargo/<crate>/`（腐蚀生成，非 maturin 产物）
- **FFI 导出库**（C FFI crate）：输出到 `${GKIT_BUILD_DIR}/lib/`
- **Python whl 包**：输出到 `${GKIT_BUILD_DIR}/dist/`（通过 `maturin build --out`）
- **Wasm 包**：输出到 `${GKIT_BUILD_DIR}/wasm/{bundler,web,nodejs}/`（通过 `wasm-pack build --out-dir`）
- **Node.js 绑定**：输出到 `${GKIT_BUILD_DIR}/node/`（`.node` 文件）
- **Flutter 绑定**：Dart 文件输出到 `${GKIT_BUILD_DIR}/flutter/<module>/dart/`，原生库到 `${GKIT_BUILD_DIR}/flutter/lib/<rid>/`
- 安装期路径：`GKIT_INSTALL_LIBDIR`, `GKIT_INSTALL_DLLDIR`, `GKIT_INSTALL_INCLUDEDIR`

实现方式：在根 `CMakeLists.txt` 的 `corrosion_import_crate` 之后，遍历所有非 C-FFI 腐蚀 target，设置输出目录到 `build/cargo/<crate>/`（Python crate 的条件与导入条件保持一致）：

```cmake
set(_gkit_redirect_crates gkit_core gkit_media gkit_native gkit_crash gkit_network gkit_service)
if(GKIT_BUILD_API_PYTHON)
    list(APPEND _gkit_redirect_crates gkit_core_py gkit_media_py)
endif()
if(GKIT_BUILD_API_WASM)
    list(APPEND _gkit_redirect_crates gkit_core_wasm gkit_media_wasm)
endif()
if(GKIT_BUILD_API_NODE)
    list(APPEND _gkit_redirect_crates gkit_core_node gkit_media_node)
endif()
if(GKIT_BUILD_API_FLUTTER)
    list(APPEND _gkit_redirect_crates gkit_core_flutter gkit_media_flutter)
endif()
foreach(_gkit_crate ${_gkit_redirect_crates})
    if(TARGET ${_gkit_crate})
        set_target_properties(${_gkit_crate} PROPERTIES
            ARCHIVE_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/cargo/${_gkit_crate}")
        if(WIN32)
            set_target_properties(${_gkit_crate} PROPERTIES
                RUNTIME_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/cargo/${_gkit_crate}")
        else()
            set_target_properties(${_gkit_crate} PROPERTIES
                LIBRARY_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/cargo/${_gkit_crate}")
        endif()
    endif()
endforeach()
```

> **注意**：C FFI crate（`gkit-core-c`, `gkit-media-c`）的输出目录由 `gkit_cargo_setup_ffi_target` 在 `apis/c/CMakeLists.txt` 中独立设置，不在上述循环中。

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
