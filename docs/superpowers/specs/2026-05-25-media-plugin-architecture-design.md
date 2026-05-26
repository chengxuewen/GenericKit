# GenericKit Media 插件架构设计

**Date**: 2026-05-25
**Status**: Design (未实施)
**Scope**: gkit-media 核心库 + stabby 跨 FFI 类型定义 + 插件发现/加载 + 测试架构 + CMake 集成

> 基于 Qt6 Multimedia 架构，采用 stabby 实现 ABI 稳定的插件系统。

---

## 1. 总体架构

**分层原则**: 插件加载器放在 `gkit-core`（类似 Qt 的 `QFactoryLoader` 在 QtCore），media 类型在 `gkit-media`。

```
gkit-core/                              # workspace 基础 (类似 QtCore)
├── src/plugin/                         # ★ 泛型插件系统，不依赖 media
│   ├── backend.rs        # PluginBackend<T> { Dynamic, Static }
│   ├── loader.rs         # PluginLoader<T> + libloading
│   ├── discovery.rs      # PluginSearchPath, scan()
│   └── error.rs          # PluginError (thiserror)
│   [deps: stabby, libloading, linkme, thiserror]
│
├── tests/plugin/                       # ★ TDD: 脱离 media 独立测试
│   └── mock_plugin/       # cdylib 导出 dummy extern "C" fn

gkit-media/                             # ★ 唯一 media crate (rlib)
├── src/
│   ├── video/frame.rs        # VideoFrame (§ stabby)
│   ├── video/buffer.rs       # I420Planes, NV12Planes 等 (§ stabby)
│   ├── trait/
│   │   ├── video_sink.rs     # IVideoSink (§ stabby trait)
│   │   ├── audio_sink.rs     # IAudioSink (§ stabby trait)
│   │   ├── webrtc.rs         # PeerConnection, data channel 等 (§ stabby trait, W3C)
│   │   └── codec.rs          # ICodec (§ stabby trait)
│   ├── plugin/
│   │   ├── registry.rs       # PluginRegistry (wraps gkit-core::PluginLoader)
│   │   └── static.rs         # linkme WASM 静态注册
│   └── error.rs              # MediaError (thiserror)
│   [deps: gkit-core, stabby]

gkit-media/plugins/                      # 插件源码 (workspace members)
├── webrtc/
│   ├── libwebrtc/            → libgkit_plugin_webrtc_libwebrtc.dylib
│   ├── webrtc-rs/            → libgkit_plugin_webrtc_rs.dylib
│   ├── gstreamer/            → libgkit_plugin_webrtc_gstreamer.dylib
│   └── web-sys/              → rlib (WASM 静态链接)
├── codec/
│   ├── ffmpeg/               → libgkit_plugin_codec_ffmpeg.dylib
│   ├── gstreamer/            → libgkit_plugin_codec_gstreamer.dylib
│   ├── avfoundation/         → macOS only
│   ├── mediacodec/           → Android only
│   └── webcodecs/            → rlib (WASM 静态链接)

cmake/
├── GKitCargoPlugin.cmake     # NEW: gkit_cargo_add_plugin()
├── GKitCargoExample.cmake
└── GKitCargoHelpers.cmake
```

**与 Qt 对照**：

| Qt | GenericKit |
|----|-----------|
| `QPlatformMediaIntegration` | `PluginRegistry` 单例 |
| `QPlatformMediaPlayer`/`QPlatformCamera` | 各后端 impl `IVideoSink`/`PeerConnection` |
| `QVideoFrame` | `VideoFrame` (§ stabby, Arc<[u8]> 零拷贝) |
| `QVideoSink` | `IVideoSink` (§ stabby trait) |
| FFmpeg 后端 = .dylib | `gkit-plugin-codec-ffmpeg` cdylib |
| `QT_MEDIA_BACKEND` env var | `GKIT_WEBRTC_BACKEND` / `GKIT_CODEC_BACKEND` |
| WASM 用 WebAudio/WebVideo | WASM 用 web-sys/WebCodecs (rlib) |

---

## 2. VideoFrame ABI (stabby)

### 2.1 最小 stabby 类型面

| stabby 类型 | 用途 |
|------------|------|
| `#[stabby::stabby]` | struct 标注 → `#[repr(C)]` + `IStable` |
| `#[repr(stabby)]` | enum Niche 紧凑布局 |
| `#[stabby::stabby]` on traits | ABI 稳定虚表 + `extern "C"` 方法 |
| `stabby::arc::Arc<T>` | 跨 dylib 引用计数 |
| `stabby::slice::Slice<'a, u8>` | 零拷贝借用 |
| `stabby::boxed::Box<T>` | ABI 稳定堆分配 |
| `stabby::dynptr!` | ABI 稳定 trait object |
| `#[stabby::export(canaries)]` | 导出函数 + 签名验证 |

### 2.2 VideoFrame 类型定义

```rust
use stabby::{arc::Arc, boxed::Box, slice::Slice};

/// 帧元数据
#[stabby::stabby]
#[derive(Debug, Clone)]
pub struct VideoFrameMeta {
    pub width: u32,
    pub height: u32,
    pub rotation: u32,
    pub timestamp_us: i64,
}

/// I420 平面缓冲 (owned, Arc 零拷贝读)
#[stabby::stabby]
pub struct I420Planes {
    pub data_y: Arc<[u8]>, pub data_u: Arc<[u8]>, pub data_v: Arc<[u8]>,
    pub stride_y: u32, pub stride_u: u32, pub stride_v: u32,
}

/// NV12 平面缓冲
#[stabby::stabby]
pub struct NV12Planes {
    pub data_y: Arc<[u8]>, pub data_uv: Arc<[u8]>,
    pub stride_y: u32, pub stride_uv: u32,
}

// ... I422、I444、I010 类似

/// 7 变体 tagged union (Niche 优化，零额外空间)
#[stabby::stabby]
pub enum BufferData {
    I420(I420Planes),
    I420A { i420: I420Planes, alpha: Arc<[u8]>, stride_alpha: u32 },
    I422 { y: Arc<[u8]>, u: Arc<[u8]>, v: Arc<[u8]>, sy: u32, su: u32, sv: u32 },
    I444 { y: Arc<[u8]>, u: Arc<[u8]>, v: Arc<[u8]>, sy: u32, su: u32, sv: u32 },
    I010 { y: Arc<[u16]>, u: Arc<[u16]>, v: Arc<[u16]>, sy: u32, su: u32, sv: u32 },
    NV12(NV12Planes),
    Native { os_handle: usize, pixel_format: u32 },
}

/// ABI 稳定 VideoFrame
#[stabby::stabby]
pub struct VideoFrame {
    pub meta: VideoFrameMeta,
    pub buffer: BufferData,
}

/// 借用变体 (零拷贝热路径)
#[stabby::stabby]
pub struct VideoFrameBorrowed<'a> { /* Slice 指针域 */ }
```

### 2.3 跨 dylib VideoSink 接口

```rust
#[stabby::stabby(checked)]
pub trait IVideoSink {
    extern "C" fn on_frame(&self, frame: Box<VideoFrame>);
    extern "C" fn on_frame_borrowed(&self, frame: &VideoFrameBorrowed<'_>);
    extern "C" fn on_discarded_frame(&self);
}
```

### 2.4 性能 (1080p@60fps I420 = 3.11 MB/frame)

| 策略 | 每帧拷贝 | Atomics | 每帧耗时 | 场景 |
|------|---------|---------|----------|------|
| Borrowed Slice | 0 | 0 | ~0ms | 单线程热路径 |
| Arc<[u8]> | 0 | 3 (每平面) | ~0.001ms | 多 sink 广播 |
| 老 C FFI 拷贝 | 3.11 MB/sink | 0 | ~0.5ms/拷贝 | 废弃 |

`Arc<[u8]>` 跨 dylib 安全: `stabby::arc::Arc` 是 `repr(C)`，控制块在创建 dylib 分配，析构函数存储在控制块内。

---

## 3. 插件加载器

### 3.1 PluginBackend<T> — 统一动态/静态

`gkit-core::plugin::backend` — 不依赖任何 media 类型，纯泛型：

```rust
pub enum PluginBackend<T> {
    Dynamic { _lib: PluginLib, instance: T },
    Static(T),
}
```

宿主代码零分支——两种变体都满足同一个 trait。`PluginLib` 在 WASM 是 ZST。

### 3.2 插件发现

```rust
pub enum PluginSearchPath {
    Directory(PathBuf),
    EnvVar(&'static str),          // GKIT_PLUGIN_PATH
    CargoTargetDir,                // target/{debug,release}/
    RelativeToExe(&'static str),   // "../plugins"
}
```

发现链: `GKIT_PLUGIN_PATH` → `CargoTargetDir` → `RelativeToExe("../plugins")` → `RelativeToExe(".")`

### 3.3 加载协议

```
1. dlopen(path)
2. 查 gkit_plugin_abi_version() → u32 (必须匹配宿主 ABI_VERSION)
3. 查 create_webrtc_backend() → Box<dyn PeerConnectionFactory>
4. 包装: PluginBackend::Dynamic { _lib: Arc<Library>, instance }
```

### 3.4 PluginRegistry

```rust
pub struct PluginRegistry {
    webrtc: RwLock<Vec<LoadedPlugin<Box<dyn PeerConnectionFactory>>>>,
    codec:  RwLock<Vec<LoadedPlugin<Box<dyn ICodecFactory>>>,
    default_order: RwLock<Vec<String>>,
}
```

**Fallback 优先级**:

| 平台 | 默认顺序 |
|------|---------|
| WASM | web-sys > webrtc-rs > libwebrtc |
| Linux ARM64 (Jetson) | libwebrtc > webrtc-rs |
| 其他 | webrtc-rs > libwebrtc |

### 3.5 WASM 静态注册 (linkme)

```rust
#[linkme::distributed_slice]
pub static WEBRTC_STATIC_PLUGINS: [fn() -> (&'static str, Box<dyn PeerConnectionFactory>)] = [..];
```

每个 WASM rlib 编译期注册，宿主通过 `PluginBackend::Static(instance)` 访问。

### 3.6 错误处理

```rust
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },
    #[error("failed to load {path}: {source}")]
    LoadFailed { path: PathBuf, source: std::io::Error },
    #[error("ABI mismatch: plugin={plugin} host={host}")]
    AbiVersionMismatch { plugin: u32, host: u32 },
    #[error("missing symbol '{symbol}' in '{name}'")]
    MissingSymbol { name: String, symbol: String },
}
```

每个加载失败 → 跳过 → 记录警告 → 尝试下一个。所有候选失败后才报错。

---

## 4. CMake 集成

### 4.1 新函数 `gkit_cargo_add_plugin`

```cmake
gkit_cargo_add_plugin(
    NAME gkit-plugin-webrtc
    CATEGORY webrtc
    FEATURES "hw-accel"
    PLATFORMS "macos;linux;windows"
    INSTALL
)
```

| 参数 | 作用 |
|------|------|
| NAME | Cargo package 名 |
| CATEGORY | 输出子目录 (`build/plugins/{CATEGORY}/`) |
| FEATURES | 传递给 `CORROSION_FEATURES` |
| PLATFORMS | 平台过滤 ("macos;linux;windows;all") |
| INSTALL | 是否添加安装规则 |

WASM 时 `gkit_cargo_add_plugin` 直接 return——不需要 post-build copy。

### 4.2 构建布局

```
build/
└── plugins/
    ├── webrtc/
    │   ├── libgkit_plugin_webrtc_libwebrtc.so
    │   └── libgkit_plugin_webrtc_rs.dylib
    └── codec/
        ├── libgkit_plugin_codec_ffmpeg.dylib
        └── libgkit_plugin_codec_avfoundation.dylib  (macOS only)
```

### 4.3 插件发现 (plugins.toml)

```toml
[plugins.webrtc-libwebrtc]
cargo_features = ["hw-accel"]
platforms = ["all"]
category = "webrtc"

[plugins.codec-avfoundation]
platforms = ["macos", "ios"]
category = "codec"
```

### 4.4 根 CMakeLists.txt 变更

```cmake
# 插件发现
include(GKitCargoPlugin)
gkit_cargo_discover_plugins(_gkit_plugin_list)
list(APPEND _gkit_corrosion_crates ${_gkit_plugin_list})

corrosion_import_crate(MANIFEST_PATH Cargo.toml CRATES ${_gkit_corrosion_crates})
add_subdirectory(plugins)
```

---

## 5. 测试架构

### 5.1 核心宏 `test_with_all_backends!`

```rust
test_with_all_backends!(create_and_close, false, |f: &dyn PeerConnectionFactory| {
    let mut pc = f.create_peer_connection()?;
    assert_eq!(pc.connection_state(), ConnectionState::New);
    pc.close()?;
});
```

编译期生成每个后端的独立 `#[test]` 函数，零代码重复。

### 5.2 测试辅助

| 模块 | 功能 |
|------|------|
| `TestPair::new(factory)` | 创建 alice+bob P2P 对 |
| `TestPair::exchange_sdp()` | 自动 offer/answer/candidate 交换 |
| `TestPair::wait_for_ice_connected(timeout)` | 等待 ICE Connected |
| `FrameCollector` | 收帧 + 断言尺寸/序列号 |
| `FrameValidator` | 像素级帧验证 |

### 5.3 测试矩阵

| 测试场景 | webrtc-rs | libwebrtc | gstreamer | web-sys |
|---------|-----------|-----------|-----------|---------|
| 生命周期 (create/close) | ✅ | ✅ | ✅ | ✅ |
| SDP 交换 | ✅ | ✅ | ✅ | ✅ |
| ICE P2P 建连 | ✅ | ✅ | ✅ | — |
| Video track 推拉 | ✅ | ✅ | ✅ | ✅ |
| DataChannel | ✅ | ✅ | ✅ | ✅ |

### 5.4 平台门控

```rust
#[cfg(all(target_os = "macos", feature = "backend-avfoundation"))]
mod avfoundation_tests { ... }
#[cfg(all(target_os = "android", feature = "backend-mediacodec"))]
mod mediacodec_tests { ... }
```

### 5.5 CI 矩阵

```yaml
strategy:
  matrix:
    os: [ubuntu-24.04, macos-14]
    webrtc-backend: [webrtc-rs, libwebrtc]
```

---

## 6. 符号隔离

每个 `.dylib` 有独立的符号表。FFmpeg 的 `libopenh264` 和 libwebrtc 的 `libopenh264` 互不可见——OS linker 保证隔离。

```
libgkit_plugin_codec_ffmpeg.dylib     libgkit_plugin_webrtc_libwebrtc.dylib
├── libavcodec 符号                    ├── libwebrtc.a 符号 (含 openh264)
├── WelsEnc::* (自己的符号表)          └── WelsEnc::* (自己的符号表)
    ↑                                       ↑
    两个 dylib 互不可见，零冲突
```

---

## 7. 迁移路径 (backward compatible)

1. `gkit-media` 现有 `RtcEngine` 委托给 `PluginRegistry::create_webrtc()`
2. 老 `gkit_register_rtc_backend!` 映射到 `PluginRegistry::register_webrtc()`
3. `engine.rs` 后续废弃
4. `core.rs` trait 保持不动 (已通过 L0/L1/L2 测试验证)

---

## 8. Cargo workspace 结构

```toml
[workspace]
members = [
    "gkit-core",
    "gkit-media",
    "gkit-media/plugins/*",
    "crates/*",
    "apis/*",
]

[workspace.dependencies]
stabby = "72"
gkit-core = { path = "gkit-core" }
gkit-media = { path = "gkit-media" }
```
