# WebRTC Backend Abstraction Layer 实现计划

## 架构

```
crates/gkit-media/
├── build.rs                           # libwebrtc download/cache/link (ureq + zip),
│                                      #   C++ bridge compilation via cc (C++20)
│                                      #   guarded by #[cfg(feature = "backend-native-google")]
├── Cargo.toml                         # features + dependencies
│                                      #   [features]: default=backend-native,
│                                      #     backend-native-webrtc-rs/backend-native-google/backend-wasm
│                                      #   [build-dependencies]: ureq 2, zip 0.6, cc 1.0, pkg-config 0.3
└── src/
    ├── lib.rs                         # pub mod webrtc; pub mod video;
    │                                  #   build_sys module currently commented out
    ├── build-sys/
    │   ├── mod.rs                     # #[path = "webrtc-sys/lib.rs"] pub mod webrtc_sys
    │   ├── webrtc-sys/                # [LiveKit 移植] unsafe FFI 绑定
    │   │   ├── lib.rs                  # crate root (24 submodules, Send + Sync guards)
    │   │   ├── *.rs (24 files)         # cxx.rs 桥接（Rust ↔ C++）
    │   │   ├── *.cpp (28 files)        # C++ 实现
    │   │   ├── *.mm (2 files)          # ObjC 视频工厂
    │   │   ├── include/livekit/ (30 .h) # C++ 头文件
    │   │   ├── nvidia/ (10 files)      # NVIDIA NVENC/NVDEC
    │   │   ├── vaapi/ (10 files)       # Linux VAAPI
    │   │   └── lazy_load_deps_for/     # Windows implib helper
    │   └── yuv-sys/                   # 单独 libyuv FFI crate
    │       ├── Cargo.toml
    │       ├── build.rs
    │       └── lib.rs
    └── webrtc/
        └── client/
            ├── mod.rs                 # cfg: native / wasm
            ├── core.rs                # W3C Trait 定义 + 枚举 + 配置 struct
            ├── wasm.rs                # Wasm 后端 stub
            └── native/
                ├── mod.rs             # cfg 选择 webrtc_rs / google（互斥）
                ├── webrtc_rs.rs        # webrtc-rs 实现（stub）
                ├── google.rs           # Google 后端 stub ← 当前默认
                └── google_lk/          # [LiveKit 移植] 安全 Rust 封装（未启用）
                    ├── mod.rs          # 23 个公开模块
                    ├── *.rs            # 安全 API 封装（共享 native+web）
                    ├── native/         # 平台实现（27 文件）
                    └── web/            # WASM 桩
```

- `gkit-media` 单 crate，全部代码内置，**不拆分独立 crate**
- `build-sys/webrtc-sys/`：LiveKit webrtc-sys 移植（cxx.rs 桥接）— **代码已全部就位**
- `webrtc/client/native/google_lk/`：LiveKit libwebrtc 移植（安全封装）— **代码已全部就位**
- 真实集成阻塞点：`lib.rs` 中 `build_sys` 模块声明被注释，需要预编译 libwebrtc 二进制

## 编译开关

```toml
[features]
default = ["backend-native"]
backend-native = []
backend-native-webrtc-rs = ["backend-native"]
backend-native-google = ["backend-native"]
backend-wasm = []
```

- `backend-native-google`：理论上使用 LiveKit 移植的安全封装（依赖 `build-sys/webrtc-sys` FFI + 预编译 libwebrtc 二进制）
- `backend-native-webrtc-rs`：使用 webrtc-rs stub（默认）
- libwebrtc 预编译二进制：
  - 由 `build.rs` 从 `https://github.com/livekit/rust-sdks/releases/download/webrtc-7af9351/` 下载
  - 缓存到平台相关目录（可覆盖 `GKIT_WEBRTC_CACHE`）
  - 可通过 `GKIT_CUSTOM_WEBRTC` 环境变量指定本地路径
  - 可通过 `GKIT_SKIP_WEBRTC_DOWNLOAD=true` 跳过下载
  - 约 300MB，首次下载后缓存复用

## 分步任务

### 第 1 步：定义 Trait 抽象 ✅
- [x] `core.rs`：PeerConnection、DataChannel traits + MediaError 等类型
- [x] `cargo check -p gkit-media` 通过

### 第 2 步：实现 Native 后端 ✅
- [x] `native/webrtc_rs.rs`：NativePeerConnection stub
- [x] `native/google.rs`：GooglePeerConnection stub → 最终应由 google_lk 替换
- [x] `native/mod.rs`：cfg 选择 + compile_error! 互斥 guard
- [x] `cargo check -p gkit-media` 通过

### 第 3 步：实现 Wasm 后端 ✅
- [x] `wasm.rs`：WasmPeerConnection stub，#[cfg(target_arch = "wasm32")]
- [x] `cargo check -p gkit-media` 通过

### 第 4 步：连接 feature flag ✅
- [x] Cargo.toml features 配置
- [x] `client/mod.rs` 条件编译
- [x] `GKIT_FEATURE_MEDIA_WEBRTC_BACKEND` CMake cache string 映射 CORROSION_FEATURES

### 第 5 步：C FFI 层适配 ✅

参考 libdatachannel `rtc.h`，扩展 W3C 标准接口。

#### 新增类型
| 类型 | 说明 | 来源 |
|------|------|------|
| `ConnectionState` | RTCPeerConnectionState | W3C / libdatachannel `rtcState` |
| `GatheringState` | RTCIceGatheringState | W3C |
| `SignalingState` | RTCSignalingState | W3C |
| `RtcConfiguration` | ICE servers + transport policy | W3C / libdatachannel `rtcConfiguration` |
| `IceServer` | STUN/TURN server entry | W3C |

#### 新增 C FFI 函数（`gkit_media_rtc_*`）

| 函数 | 说明 |
|------|------|
| `peer_connection_connection_state` | 连接状态（0=New..5=Closed） |
| `peer_connection_gathering_state` | ICE 收集状态（0=New,1=Gathering,2=Complete） |
| `peer_connection_signaling_state` | 信令状态（0=Stable..4=HaveRemotePranswer） |
| `peer_connection_get_local_description` | 获取本地 SDP |
| `peer_connection_get_remote_description` | 获取远端 SDP |

**总计 42 个 `gkit_media_rtc_*` + 14 个 `gkit_media_video_frame_*` extern "C" 函数。**

### 第 6 步：测试覆盖 ✅

**三层测试结构**：

#### Rust Trait 层（`crates/gkit-media/tests/`）
- 5 文件 21 tests（basic, sdp, data_channel, states, errors）
- 后端无关：使用 `gkit_media::make_peer_connection()`
- `cargo test -p gkit-media`（默认 webrtc-rs）
- `cargo test -p gkit-media --features backend-native-google`

#### C 语言 FFI 层（`apis/c/gkit-media/tests/`）
- 5 文件 C 源代码，使用 Unity 测试框架（`#include "unity.h"`）
- `setUp()` / `tearDown()` 管理共享资源生命周期
- CMake 构建：`add_executable` + `target_link_libraries(gkit_media_c GKitWrapUnity::WrapUnity)`
- IDE FOLDER：`gkit_media/apis/c/tests`
- CTest 注册：5 个 C 可执行测试

| C 测试文件 | 覆盖 |
|-----------|------|
| `test_basic.c` | create/destroy, multiple, null-safe |
| `test_sdp.c` | offer/answer round-trip, ICE candidate, ICE state |
| `test_data_channel.c` | label, send_text, send_bytes, close, error-on-closed |
| `test_errors.c` | null handles, closed peer rejections |
| `test_video_frame.c` | VideoFrame create/destroy, scale/crop/rotate |

#### C++ FFI 层（`apis/cpp/gkit-media/tests/`）
- 1 文件 C++ 源代码，使用 GTest 框架
- IDE FOLDER：`gkit_media/apis/cpp/tests`

#### CTest 集成（CMake 根）
| 名称 | 类型 | 命令 |
|------|------|------|
| `gkit_media_tests_native` | Cargo test | `cargo test -p gkit-media --features backend-native` |
| `gkit_media_tests_google` | Cargo test | `cargo test -p gkit-media --features backend-native-google` + `GKIT_SKIP_WEBRTC_DOWNLOAD=true` |
| `gkit_media_c_test_basic` | C executable | `apis/c/gkit-media/tests/test_basic.c` |
| `gkit_media_c_test_sdp` | C executable | `apis/c/gkit-media/tests/test_sdp.c` |
| `gkit_media_c_test_dc` | C executable | `apis/c/gkit-media/tests/test_data_channel.c` |
| `gkit_media_c_test_errors` | C executable | `apis/c/gkit-media/tests/test_errors.c` |
| `gkit_media_c_test_video_frame` | C executable | `apis/c/gkit-media/tests/test_video_frame.c` |
| `gkit_media_cpp_test_video_frame` | C++ executable | `apis/cpp/gkit-media/tests/test_video_frame.cpp` |

```bash
ctest --test-dir build                       # 运行全部 8 项
ctest -R gkit_media                          # 过滤 WebRTC 相关
```

#### 验证

- [x] `cargo test -p gkit-media`：21/21 通过（webrtc-rs + google stubs）
- [x] C 测试：5 个可执行文件通过 `add_test` 注册 CTest
- [x] C++ 测试：1 个可执行文件通过 `add_test` 注册 CTest
- [x] C 测试代码纯 C 语言，`#include "gkit_media.h"` 验证头文件可用性
- [x] C++ 测试代码纯 C++，`#include <gkit_media_video_frame.hpp>` 验证头文件可用性

### 第 7 步：移植 LiveKit webrtc-sys + libwebrtc 🔄

| 组件 | 来源 | 状态 | 目标路径 |
|------|------|------|----------|
| Rust FFI 桥接 | webrtc-sys/src/*.rs | ✅ 已复制 24 文件 | `build-sys/webrtc-sys/` |
| C++ 实现 | webrtc-sys/src/*.cpp | ✅ 已复制 28 文件 | `build-sys/webrtc-sys/` |
| ObjC 文件 | webrtc-sys/src/*.mm | ✅ 已复制 2 文件 | `build-sys/webrtc-sys/` |
| C++ 头文件 | webrtc-sys/include/ | ✅ 已复制 30 文件 | `build-sys/webrtc-sys/include/livekit/` |
| NVIDIA codec | webrtc-sys/src/nvidia/ | ✅ 已复制 10 文件 | `build-sys/webrtc-sys/nvidia/` |
| VA-API codec | webrtc-sys/src/vaapi/ | ✅ 已复制 10 文件 | `build-sys/webrtc-sys/vaapi/` |
| 安全 API 封装 | libwebrtc/src/*.rs | ✅ 已复制 23 文件 | `google_lk/` |
| 安全实现层 | libwebrtc/src/native/*.rs | ✅ 已复制 27 文件 | `google_lk/native/` |
| WASM 桩 | libwebrtc/src/web/*.rs | ✅ 已复制 3 文件 | `google_lk/web/` |
| `build-sys/mod.rs` | 新建 | ✅ 已创建 | `#[path = "webrtc-sys/lib.rs"] pub mod webrtc_sys` |
| build.rs | 适配 | ✅ 已完成 | 下载 + 缓存 + C++20 编译 + 平台链接，由 `backend-native-google` 守卫 |
| lib.rs 模块声明 | — | ⏳ 已注释 | `pub mod build_sys` 被注释 |

#### 当前阻塞项

```
// crates/gkit-media/src/lib.rs — 当前状态：
// build_sys 模块被注释，实际集成未完成
//
// #[cfg(feature = "backend-native-google")]
// #[path = "build-sys/mod.rs"]
// pub mod build_sys;
```

#### 集成待办项

- [ ] 取消注释 `lib.rs` 中 `pub mod build_sys` 声明
- [ ] 将 `google.rs` stub 替换为 `google_lk/` 中真实封装
- [ ] 修复 `google_lk/native/` 中模块路径适配（如 `use webrtc_sys::*` 引用路径）
- [ ] 更新 `Cargo.toml` 添加 `backend-native-google` 所需依赖（cxx, cc, glob 等）
- [ ] `cargo check -p gkit-media --features backend-native-google` 通过
- [ ] 提供预编译 libwebrtc 二进制（`GKIT_CUSTOM_WEBRTC` 路径或允许下载）

## 自动化测试与构建

### 构建目录

CMake 构建目录统一使用 `build-auto`，自动化测试脚本默认从此目录构建：

```bash
cmake -B build-auto -S . -DGKIT_BUILD_TESTS=ON
cmake --build build-auto
ctest --test-dir build-auto
```

### 测试命令

```bash
# C FFI 层 (Unity, 5 tests)
cmake --build build-auto --target gkit_media_c_test_basic gkit_media_c_test_sdp ...
ctest --test-dir build-auto -R gkit_media_c_test

# C++ FFI 层 (GTest, 1 test)
cmake --build build-auto --target gkit_media_cpp_test_video_frame
ctest --test-dir build-auto -R gkit_media_cpp_test

# Rust trait 层 (21 tests)
cargo test -p gkit-media                                            # webrtc-rs backend
cargo test -p gkit-media --features backend-native-google           # Google backend (needs libwebrtc download)

# 全部测试
ctest --test-dir build-auto && cargo test -p gkit-media
```

### 当前测试状态

| 测试层 | 命令 | 数量 | 结果 |
|--------|------|------|------|
| C FFI | `ctest -R gkit_media_c_test` | 5 | ✅ 100% |
| C++ FFI | `ctest -R gkit_media_cpp_test` | 1 | ✅ 100% |
| Trait webrtc-rs | `cargo test -p gkit-media` | 21 | ✅ 100% |
| Trait google | `cargo test -p gkit-media --features backend-native-google` | 21 | ⏳ stub 通过，真实需 libwebrtc |

## yuv-sys（独立 FFI crate）

`build-sys/yuv-sys/` 是一个独立的 Cargo crate，提供 libyuv 的 unsafe FFI 绑定：

```
crates/gkit-media/src/build-sys/yuv-sys/
├── Cargo.toml        # 独立 crate，非 workspace member
├── build.rs          # 编译 C 源文件
├── lib.rs            # 声明 YUV 函数
└── yuv_functions.txt # 函数列表
```

注意：yuv-sys 不在 workspace 根 `Cargo.toml` 的 `members` 中，但作为 `gkit-media` 的本地 path dependency。

## 注意事项

- **不创建独立 crate**：全部代码在 `gkit-media` 内部，避免 workspace 膨胀
- **libwebrtc 由 build.rs 管理**：prebuilt `libwebrtc.a` 通过 GitHub release 下载，或通过 `GKIT_CUSTOM_WEBRTC` 环境变量传入本地路径
- **Google 后端**：首次启用需下载 libwebrtc (~300MB)，后续使用缓存（Linux `~/.cache/gkit_webrtc/`, macOS `~/Library/Caches/gkit_webrtc/`, Windows `%LOCALAPPDATA%\gkit_webrtc\`）
- **build.rs 守卫**：仅在 `#[cfg(feature = "backend-native-google")]` 且非 docs/check 时执行
- **Wasm 平台**：`GKIT_FEATURE_MEDIA_WEBRTC_BACKEND=wasm` 强制且唯一可用
- **Google 后端 build.rs 平台链接**：
  - macOS: 16 个系统框架（AudioToolbox, VideoToolbox, CoreMedia 等）
  - iOS: 15 个框架 + 3 系统库
  - Linux: dl, pthread, rt, X11, GL, Xext
  - Windows: ws2_32, winmm, secur32, dmoguids, wmcodecdspuuid, msdmo, strmiids
