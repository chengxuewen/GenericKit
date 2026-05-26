# GenericKit Media 插件架构设计

**Date**: 2026-05-25
**Status**: Implemented (已实施 P0-P5)

> **实施进度** (2026-05-26):
> - P0 ✅: gkit-core 通用插件加载器 (PluginLib, PluginLoader, PluginDiscovery)
> - P1 ✅: gkit-media stabby 类型定义 + IStableVideoSink + IStablePeerConnectionFactory
> - P2 ✅: gkit-media PluginRegistry<T> + TDD-4 测试
> - P3 ✅: 第一个 cdylib 插件 (gkit-plugin-webrtc-libwebrtc)
> - P4 ✅: RtcEngine 集成 PluginRegistry + load_plugins() 动态发现/加载
> - P5 ✅: WASM web-sys 插件 → plugins/webrtc/web-sys/ (rlib 静态链接)
> - **已移除**: `backend-native-google` feature + `livekit_rs` 模块 + `native/` 目录
> - **已移除**: `backend-native-webrtc-rs` feature + `webrtc_rs.rs`
> - **已移除**: gkit-media 的 `wasm.rs` → web-sys plugin

---

## 1. 总体架构

**分层原则**: 插件加载器放在 `gkit-core`（类似 Qt 的 `QFactoryLoader` 在 QtCore），media 类型在 `gkit-media`。

```
gkit-core/                              # workspace 基础 (类似 QtCore)
│   [路径: crates/gkit-core/ — 保持现有结构]
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
│   [路径: crates/gkit-media/]
├── src/
│   ├── video/
│   │   ├── frame.rs           # VideoFrame<T> (generic, non-stabby)
│   │   ├── frame_stabby.rs    # StableVideoFrame, I420Planes, NV12Planes, BufferData
│   │   ├── buffer.rs          # I420/I422/I444/NV12/I010 buffers
│   │   ├── source_sink.rs     # VideoSink<F>, VideoSource<F>, VideoBroadcaster
│   │   ├── convert.rs / transform.rs / adapter.rs
│   ├── trait/
│   │   ├── video_sink_stabby.rs  # IStableVideoSink (on_frame/on_frame_owned)
│   │   └── webrtc_stabby.rs      # IStablePeerConnectionFactory (backend_name)
│   ├── plugin/
│   │   └── registry.rs           # PluginRegistry<T> (wraps gkit-core)
│   └── protocols/rtc/client/
│       ├── core.rs                # PeerConnection, DataChannel, VideoTrack traits
│       ├── engine.rs              # RtcEngine + load_plugins() + PluginRegistry 集成
│       ├── engine_macros.rs       # gkit_register_rtc_backend! 宏 (WASM 静态注册)
│       └── mod.rs
│   [deps: gkit-core, stabby, ctor(optional)]
│
gkit-media/plugins/                      # 插件 (workspace members)
└── webrtc/
    ├── libwebrtc/          → libgkit_plugin_webrtc_libwebrtc.dylib (cdylib)
    │   └── src/adapt/      LiveKit rust-sdks 适配 (12 modules + convert.rs)
    │   [deps: gkit-media, libwebrtc, tokio]
    └── web-sys/            → rlib (WASM 静态链接)
        └── src/lib.rs      WasmPeerConnection + WasmFactory + #[ctor] 注册
        [deps: gkit-media, ctor]

cmake/
├── GKitCargoPlugin.cmake     # gkit_cargo_add_plugin() + gkit_cargo_setup_plugins()
├── GKitCargoExample.cmake
└── GKitCargoHelpers.cmake
```

**与 Qt 对照**：

| Qt | GenericKit |
|----|-----------|
| `QPlatformMediaIntegration` | `PluginRegistry` 单例 |
| `QPlatformMediaPlayer`/`QPlatformCamera` | 各后端 impl `IVideoSink`/`PeerConnection` |
| `QVideoFrame` | `VideoFrame` (§ stabby, Arc<[u8]> 零拷贝) |
| `QVideoSink` | `IStableVideoSink` (§ stabby trait) |
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
pub trait IStableVideoSink {
    /// 广播路径 — source 默认调用此方法。
    /// 多 sink 可同时收到同一帧的引用，sink 可 clone 内部 Arc<[u8]> 持有。
    /// Caller must NOT hold the reference beyond this call — Arc clone if needed.
    extern "C" fn on_frame(&self, frame: &VideoFrame);
    /// 独占路径 — 当 sink 作为唯一消费者时调用（如编码器输入）。
    /// 传递所有权，避免 clone 开销。
    extern "C" fn on_frame_owned(&self, frame: stabby::boxed::Box<VideoFrame>);
    extern "C" fn on_discarded_frame(&self, timestamp_us: i64);
}
```
**分发约定**: source 默认广播时调用 `on_frame`（多 sink）。sink 通过 capability flag 声明支持 `on_frame_owned`，source 仅在单 sink 场景调用它。详见 trait 文档。

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
pub struct PluginLib(pub(crate) Arc<Library>);

// 注意: instance 必须在 _lib 之前声明，确保 Drop 顺序正确
// Rust 按声明顺序 drop enum 变体字段 — instance 先析构（dylib 代码仍可用），
// _lib 后析构（安全释放 dylib 句柄）
pub enum PluginBackend<T> {
    Dynamic { instance: T, _lib: PluginLib },  // ✅ instance drops FIRST
    Static(T),
}

impl<T> PluginBackend<T> {
    pub fn instance(&self) -> &T {
        match self {
            Self::Dynamic { instance, .. } | Self::Static(instance) => instance,
        }
    }
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

### 3.5 WASM 静态注册 (inventory)

**linkme 不支持 WASM**（第 3 轮审查发现）。改用 `inventory` crate（同作者 dtolnay，支持 WASM）：

```rust
// 宿主声明
inventory::collect!(WasmWebrtcPlugin);

pub struct WasmWebrtcPlugin {
    pub name: &'static str,
    pub factory: fn() -> Box<dyn PeerConnectionFactory>,
}

// 每个 WASM rlib 注册:
inventory::submit! {
    WasmWebrtcPlugin { name: "web-sys", factory: || Box::new(WebSysFactory) }
}
```

`inventory` 在 WASM 上通过 `ctor`/`init_array` 段实现，比 linkme 多一个间接层但支持 WASM。

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

### 4.3 插件声明 (CMake 原生格式，非 TOML)

由于 CMake 无法解析 TOML，采用函数调用方式在 plugins/CMakeLists.txt 中声明 (审查发现 FATAL #2)：

```cmake
# plugins/CMakeLists.txt
gkit_cargo_add_plugin(
    NAME gkit-plugin-webrtc-libwebrtc
    CATEGORY webrtc
    FEATURES "hw-accel"
    PLATFORMS macos linux windows
    INSTALL
)

gkit_cargo_add_plugin(
    NAME gkit-plugin-codec-avfoundation
    CATEGORY codec
    PLATFORMS macos ios
)
```

`gkit_cargo_add_plugin` 不创建 CMake target — 它在 `corrosion_import_crate` **之前**调用，仅追加 crate 名到 `_gkit_corrosion_crates` 列表。Corrosion 负责构建，此函数负责配置 (目录、feature、平台过滤、安装规则)。

### 4.4 Feature Flags 与 WASM 路径 `[target.'cfg(...)'.lib]` 条件编译 (审查发现 FATAL #3)。所有插件统一 `crate-type = ["cdylib", "rlib"]`。WASM 路径通过 **feature flags** 控制：
```toml
[features]
wasm-backends = ["dep:gkit-plugin-webrtc-web-sys", "dep:gkit-plugin-codec-webcodecs"]
```
Native 构建时不开启 `wasm-backends`，插件独立为 cdylib。WASM 构建时开启此 feature，插件作为 rlib 静态链接。

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

### 5.6 TDD 开发场景 (按实施顺序)

#### TDD-1: gkit-core 插件发现 (TDD: RED→GREEN)

**测试目标**: 扫描目录找到 mock `.dylib` 插件文件。

```rust
// gkit-core/tests/plugin/discovery.rs
#[test]
fn scan_empty_directory_returns_none() {
    let dir = tempdir();
    let plugins = PluginDiscovery::scan(dir.path()).unwrap();
    assert!(plugins.is_empty());
}

#[test]
fn scan_finds_dylib_plugins() {
    let dir = tempdir();
    // 创建 mock .dylib 文件
    let plugin_path = dir.path().join("libgkit_plugin_test.dylib");
    std::fs::write(&plugin_path, b"mock dylib").unwrap();
    
    let plugins = PluginDiscovery::scan(dir.path()).unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].name, "test");
}

#[test]
fn scan_ignores_non_plugin_files() {
    let dir = tempdir();
    std::fs::write(dir.path().join("readme.txt"), b"hello").unwrap();
    std::fs::write(dir.path().join("librandom.dylib"), b"not gkit").unwrap();
    
    let plugins = PluginDiscovery::scan(dir.path()).unwrap();
    assert!(plugins.is_empty());
}

#[test]
fn plugin_search_path_resolves_cargo_target_dir() {
    let path = PluginSearchPath::CargoTargetDir;
    let dirs = path.resolve();
    assert!(!dirs.is_empty());
}

#[test]
fn plugin_search_path_env_var_fallback() {
    std::env::set_var("GKIT_PLUGIN_PATH", "/tmp/nonexistent");
    let path = PluginSearchPath::EnvVar("GKIT_PLUGIN_PATH");
    let dirs = path.resolve();
    assert_eq!(dirs.len(), 1);
}
```

#### TDD-2: gkit-core 插件加载 (TDD: RED→GREEN)

**前置**: 创建 mock plugin cdylib 导出稳定符号。

```rust
// gkit-core/tests/plugin/mock_plugin/src/lib.rs (cdylib)
#[stabby::export]
pub extern "C" fn gkit_plugin_abi_version() -> u32 { 1 }

#[stabby::export(canaries)]
pub extern "C" fn create_mock_backend() -> u32 { 42 }
```

```rust
// gkit-core/tests/plugin/loader.rs
#[test]
fn load_mock_plugin_gets_abi_version() {
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    let version: Symbol<extern "C" fn() -> u32> = unsafe { lib.get(b"gkit_plugin_abi_version").unwrap() };
    assert_eq!(version(), 1);
}

#[test]
fn load_mock_plugin_with_stabby_type_check() {
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    let create = unsafe {
        lib.get_stabbied::<extern "C" fn() -> u32>(b"create_mock_backend")
    }.unwrap();
    assert_eq!(create(), 42);
}

#[test]
fn abi_version_mismatch_is_detected() {
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    let version: Symbol<extern "C" fn() -> u32> = unsafe { lib.get(b"gkit_plugin_abi_version").unwrap() };
    assert_ne!(version(), 999); // 模拟版本不匹配
}

#[test]
fn missing_symbol_returns_error() {
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    let result: Result<Symbol<extern "C" fn() -> u32>, _> = unsafe { lib.get(b"nonexistent_symbol") };
    assert!(result.is_err());
}
```

#### TDD-3: PluginBackend Drop 顺序 (TDD: RED→GREEN)

```rust
// gkit-core/tests/plugin/backend.rs
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct DropTracker {
    dropped: Arc<AtomicBool>,
}
impl Drop for DropTracker {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

#[test]
fn dynamic_backend_drops_instance_before_library() {
    let dropped = Arc::new(AtomicBool::new(false));
    let tracker = DropTracker { dropped: dropped.clone() };
    let lib = unsafe { Library::new(mock_plugin_path()).unwrap() };
    
    let backend = PluginBackend::Dynamic {
        _lib: PluginLib(Arc::new(lib)),
        instance: tracker,
    };
    drop(backend);
    // tracker.instance 应先析构，_lib 后析构
    // 如果顺序反了，dropped 在 _lib 释放后才设置 = use-after-free
    assert!(dropped.load(Ordering::SeqCst));
}

#[test]
fn static_backend_works() {
    let backend = PluginBackend::Static(42u32);
    assert_eq!(*backend.instance(), 42);
}
```

#### TDD-4: gkit-media PluginRegistry fallback 链 (TDD: RED→GREEN)

```rust
// gkit-media/tests/plugin/registry.rs
#[test]
fn registry_returns_none_when_empty() {
    let registry = PluginRegistry::new();
    let result = registry.create_webrtc(None);
    assert!(result.is_err());
}

#[test]
fn registry_fallback_to_next_when_first_fails() {
    let registry = PluginRegistry::new();
    // 注册一个会失败的 backend
    registry.set_default_order(vec!["fail".into(), "ok".into()]);
    // "fail" 不存在 → fallback 到 "ok" 
    // (需要 mock 注册表来模拟)
}

#[test]
fn registry_returns_error_when_all_fail() {
    let registry = PluginRegistry::new();
    registry.set_default_order(vec!["fail1".into(), "fail2".into()]);
    let result = registry.create_webrtc(None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no webrtc backend"));
}
```

#### TDD-5: VideoFrame stabby 往返 (TDD: RED→GREEN)

```rust
// gkit-media/tests/video/video_frame_stabby.rs
#[test]
fn i420_frame_roundtrip_preserves_dimensions() {
    let frame = VideoFrame {
        meta: VideoFrameMeta { width: 640, height: 480, rotation: 0, timestamp_us: 0 },
        buffer: BufferData::I420(I420Planes {
            data_y: Arc::from(vec![128u8; 640*480].into_boxed_slice()),
            data_u: Arc::from(vec![64u8; 320*240].into_boxed_slice()),
            data_v: Arc::from(vec![64u8; 320*240].into_boxed_slice()),
            stride_y: 640, stride_u: 320, stride_v: 320,
        }),
    };
    assert_eq!(frame.meta.width, 640);
    assert_eq!(frame.meta.height, 480);
}

#[test]
fn nv12_frame_roundtrip() {
    let frame = VideoFrame {
        meta: VideoFrameMeta { width: 1920, height: 1080, rotation: 0, timestamp_us: 0 },
        buffer: BufferData::NV12(NV12Planes {
            data_y: Arc::from(vec![128u8; 1920*1080].into_boxed_slice()),
            data_uv: Arc::from(vec![64u8; 1920*540].into_boxed_slice()),
            stride_y: 1920, stride_uv: 1920,
        }),
    };
    assert!(matches!(frame.buffer, BufferData::NV12(_)));
}

#[test]
fn arc_reference_count_increments_on_clone() {
    let data = Arc::from(vec![1u8, 2, 3].into_boxed_slice());
    let clone1 = Arc::clone(&data);
    let clone2 = Arc::clone(&data);
    drop(clone1);
    drop(clone2);
    // 原始 data 仍可用 — ref count 正确管理
    assert_eq!(&data[..], &[1u8, 2, 3]);
}
```

#### TDD-6: IStableVideoSink 跨线程广播 (TDD: RED→GREEN)

```rust
// gkit-media/tests/trait/video_sink_stabby.rs
use std::sync::{Arc, Mutex};

struct CountingSink {
    count: Mutex<u32>,
}
impl IStableVideoSink for CountingSink {
    extern "C" fn on_frame_owned(&self, _frame: Box<VideoFrame>) {
        *self.count.lock().unwrap() += 1;
    }
    extern "C" fn on_frame(&self, _frame: &VideoFrame) {}
    extern "C" fn on_discarded_frame(&self, _timestamp_us: i64) {}
}

#[test]
fn sink_counts_frames() {
    let sink = CountingSink { count: Mutex::new(0) };
    let frame = make_test_i420_frame(640, 480);
    sink.on_frame_owned(Box::new(frame));
    assert_eq!(*sink.count.lock().unwrap(), 1);
}

#[test]
fn multiple_frame_receives_increment_correctly() {
    let sink = CountingSink { count: Mutex::new(0) };
    for _ in 0..5 {
        sink.on_frame_owned(Box::new(make_test_i420_frame(320, 240)));
    }
    assert_eq!(*sink.count.lock().unwrap(), 5);
}
```

#### TDD-7: P2P 集成 — 动态加载后端 (TDD: GREEN—需真实 dylib)

```rust
// gkit-media/tests/integration/p2p_with_dynamic_plugin.rs
#[test]
#[ignore = "requires compiled plugin dylib"]
fn p2p_with_dynamically_loaded_backend() {
    // 1. 构建插件 dylib
    // 2. 宿主 dlopen + get_stabbied 加载
    // 3. TestPair + P2P exchange + ICE
    // 4. 验证 VideoTrack 推拉流
}

#[test]
fn static_backend_p2p_works() {
    // 使用编译期注册的 backend (gkit_register_rtc_backend! 或 linkme)
    let factory = RtcEngine::create("webrtc-rs").unwrap();
    let mut pair = TestPair::new(&*factory);
    pair.exchange_sdp();
    pair.wait_for_ice_connected(Duration::from_secs(15)).unwrap();
}
```

#### TDD-8: WASM 静态注册 (TDD: RED→GREEN, 需 WASM 目标)

```rust
// gkit-media/tests/wasm/static_registration.rs
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn wasm_static_plugin_registers_correctly() {
    // linkme distributed_slice 在 WASM 链接后应有至少一个 entry
    let plugins = PluginRegistry::static_plugins();
    assert!(!plugins.is_empty());
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn wasm_static_backend_creates_pc() {
    let factory = PluginRegistry::create_webrtc(None).unwrap();
    let pc = factory.create_peer_connection().unwrap();
    assert!(matches!(pc.connection_state(), ConnectionState::New));
    pc.close().unwrap();
}
```

### 5.7 TDD 实施顺序

| 阶段 | 测试文件 | 依赖 | 可并行 |
|------|---------|------|--------|
| P0 | `gkit-core/tests/plugin/discovery.rs` | 无 | ✅ |
| P0 | `gkit-core/tests/plugin/loader.rs` | P0 discovery | ✅ |
| P0 | `gkit-core/tests/plugin/backend.rs` | P0 loader | ✅ |
| P1 | `gkit-media/tests/video/video_frame_stabby.rs` | 无 | ✅ |
| P1 | `gkit-media/tests/trait/video_sink_stabby.rs` | P1 video_frame | — |
| P2 | `gkit-media/tests/plugin/registry.rs` | P0 backend | — |
| P3 | `gkit-media/tests/integration/p2p_with_dynamic_plugin.rs` | P1 + P2 | — |
| P4 | `gkit-media/tests/wasm/static_registration.rs` | 需 WASM target | ✅

```yaml
### 5.8 现有测试迁移方案

现有 16 个测试文件，分类如下：

| 类别 | 文件数 | 当前后端用法 | 迁移影响 |
|------|--------|------------|---------|
| **A: 纯视频** | 5 | 无 RTC 依赖（video_frame_*, test_source_sink） | **零改动** |
| **B: `make_peer_connection()`** | 5 | webrtc_basic/states/data_channel/errors/offer_answer | **不改**（如 make_peer_connection 更新为插件发现） |
| **C: `RtcEngine::create_default()`** | 4 | webrtc_p2p/p2p_conn/track/ice | **不改**（如 create_default 更新） |
| **D: `RtcEngine::create("google")`** | 2 | webrtc_lk_basic, webrtc_lk_p2p | **必须改写** → 插件文件路径 |
| **E/F: async + platform gates** | — | `#[tokio::test]` / `#[ignore]` / `#[cfg]` | runtime 检查替代编译期 gate |

**安全的迁移路径**：`RtcEngine` API 保持不动，内部委托给 `PluginLoader`。这样 14/16 测试文件无需修改测试源码——但 `RtcEngine::create/register/registered_types` 三个核心方法内部需完全重写。仅 `webrtc_lk_*` 两个文件需要转向插件文件路径（`"google" → "libwebrtc"`）。

另外 4 个 inline `#[cfg(test)]` 单元测试 (`livekit_rs/` 下的 `peer_connection/ice/stats/session_description`) 也依赖 `RtcEngine` 或 livekit 工厂，需同步审查。

**现有 `#[ignore]`/`#[cfg]` 平台门控** → 插件加载器返回明确错误替代编译期 skip，使测试在所有平台上可运行（跳过 vs 失败 vs 通过）。

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

## 7. 迁移路径 (修正版)

原设计 "core.rs trait 保持不动" 与引入 stabby trait 矛盾 (FATAL #1)。修正方案 (合并原 Section 9.3)：

1. **老 trait 保留**: `VideoSink<F>`, `PeerConnection` 等继续在 `core.rs` 中
2. **新 stabby trait 平行**: `IStableVideoSink`, `IStablePeerConnection` 在 `gkit-media/src/trait/`
3. **适配层**: `PluginRegistry` 返回 `Box<dyn IStablePeerConnection>`，`From`/`Into` 转换到老 trait
4. **RtcEngine 过渡**: 先查静态注册表 → fallback 到 `PluginRegistry`

**第一个后端插件**: `gkit-media/plugins/webrtc/libwebrtc/`

依赖 LiveKit rust-sdks 的 `webrtc-sys` (CXX FFI) 和 `libwebrtc` (safe wrapper)。可复用已完成迁移的 `livekit_rs/` 代码——该 adapter 已实现 `PeerConnection` trait 并经过 17 个测试验证。迁移为 cdylib 插件只需：加 `#[stabby::export]` 导出函数、改 `crate-type = ["cdylib", "rlib"]`。其他后端按需随后实施。

### 7.1 示例迁移

| 示例 | RTC 依赖 | 迁移影响 |
|------|---------|---------|
| `gkit-media-webrtc-loopback` (egui P2P) | `RtcEngine::create(&backend)` + `registered_types()` | 更新 `registered_types()` 为插件目录扫描；CMake FEATURES 改为构建插件 dylib |
| `gkit-media-viewer` (视频变换) | 无 | 零改动 |
| `gkit-media-square-gen` (帧生成) | 无 | 零改动 |

### 7.2 CMake 示例构建变更

新增 `gkit_cargo_add_plugin()` 宏（Section 4.1），用于构建 cdylib 后端插件。webrtc-loopback 的 CMakeLists.txt 从静态 feature flag → 依赖插件构建目标：

```cmake
# 旧 (静态链接):
set(_loopback_features "backend-native-webrtc-rs")
gkit_cargo_add_example(... FEATURES "${_loopback_features}")

# 新 (插件发现):
gkit_cargo_add_example(NAME gkit-media-webrtc-loopback ...)  # 无 FEATURES
# 插件 dylib 由 gkit_cargo_add_plugin 构建到 build/plugins/webrtc/
# 添加 POST_BUILD 步骤复制 dylib 到示例可执行文件旁:
add_custom_command(TARGET gkit-media-webrtc-loopback POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${CMAKE_BINARY_DIR}/plugins/webrtc/libgkit_plugin_webrtc_rs.dylib"
        "$<TARGET_FILE_DIR:gkit-media-webrtc-loopback>/plugins/"
    DEPENDS gkit_plugin_webrtc_rs)
```

---

## 8. Cargo workspace 结构

```toml
[workspace]
members = [
    "crates/*",
    "crates/gkit-media/plugins/*",
    "apis/c/*",
    "apis/python/*",
    "apis/wasm/*",
    "apis/node/*",
    "apis/flutter/*",
    "apis/csharp/*",
    "tools/gkit-vcpkg",
]

[workspace.dependencies]
stabby = "72.1"
libloading = "0.8"
inventory = "0.3"
thiserror = "2"
gkit-core = { path = "crates/gkit-core" }
gkit-media = { path = "crates/gkit-media" }
```

> **注意**: `plugins/*` 目录需预创建（含 `.gitkeep`）避免 Cargo 报错。
> gkit-core 的 plugin 依赖通过 feature flag `plugin` 控制，不传播到下游。

---

## 9. 运行时安全 & 迁移路径 (审查修正)

### 9.1 Arc 跨 dylib 安全 + 不卸载策略

`stabby::arc::Arc<[u8]>` 控制块在创建 dylib 分配，析构函数存储在控制块内。插件**永不卸载** — `PluginLib` 持有 `Arc<Library>`，生命周期 = 进程生命周期 (FATAL #2 修正)。

### 9.2 tokio runtime 隔离

每个 `.dylib` 插件自带独立 tokio runtime。宿主 async 上下文通过 channel 触发插件操作，避免嵌套 runtime panic (FATAL #3 修正)。

### 9.3 迁移路径 (FATAL #1 修正)

原设计 "core.rs trait 保持不动" 与引入 stabby trait 矛盾。修正：

1. **老 trait 保留**: `VideoSink<F>`, `PeerConnection` 等继续在 `core.rs` 中
2. **新 stabby trait 平行**: `IStableVideoSink`, `IStablePeerConnection` 在 `gkit-media/src/trait/`
3. **适配层**: `PluginRegistry` 返回 `Box<dyn IStablePeerConnection>`，`From`/`Into` 转换到老 trait
4. **RtcEngine 过渡**: 先查静态注册表 → fallback 到 `PluginRegistry`

### 9.4 构建系统修正摘要

| 审查问题 | 修正 |
|---------|------|
| F1: gkit_cargo_add_plugin 模糊 | 明确为 pre-import 配置函数 |
| F2: TOML 不可行 | CMake 函数调用 (Section 4.3) |
| F3: cfg on [lib] 语法错误 | feature flags `wasm-backends` (Section 4.4) |
