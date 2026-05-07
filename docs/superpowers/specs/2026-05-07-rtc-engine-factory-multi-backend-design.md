# RtcEngine 多后端工厂模式设计

**Date**: 2026-05-07
**Scope**: RtcEngine 全局注册中心、PeerConnectionFactory 对象安全化、多后端共存、google_lk 后端激活、C FFI/C++ 工厂封装
**Reference**: [OpenCTK RtcEngine](https://gitee.com/chengxuewen/OpenCTK/blob/master/src/libs/media/source/protocols/rtc/octk_rtc_engine.hpp) — 工厂模式 + 静态自注册宏
**Constraint**: 所有代码在单个 `gkit-media` crate，不新增 workspace member

> **Prerequisite**: [WebRTC backend spec](2026-05-06-webrtc-backend-implementation-design.md) — webrtc-rs 后端 + callback 系统已完成

---

## 1. 架构与模块布局

```
crates/gkit-media/src/
├── protocols/rtc/client/
│   ├── core.rs                        # [CHANGE] PeerConnectionFactory 去掉关联类型
│   ├── engine.rs                      # [NEW] RtcEngine — 全局注册中心 + 创建入口
│   ├── engine_macros.rs               # [NEW] gkit_register_rtc_backend! 宏
│   ├── native/
│   │   ├── mod.rs                     # [CHANGE] 移除互斥 compile_error!，两后端可同时编译
│   │   ├── webrtc_rs.rs               # [CHANGE] 实现新的 PeerConnectionFactory (无关联类型)
│   │   ├── google.rs                  # [REWRITE] 包装 google_lk 真实实现，实现新 Factory
│   │   └── google_lk/                 # [UNCHANGED] LiveKit 移植代码
│   └── wasm.rs                        # [CHANGE] 实现新的 Factory (无关联类型)
├── lib.rs                             # [CHANGE] make_peer_connection() 改用 RtcEngine
└── build-sys/                         # [UNBLOCK] 解除注释，仅 google feature 编译
```

| 模块 | 职责 |
|------|------|
| `engine.rs` | `RtcEngine` — 全局 `HashMap<&str, fn()->Box<dyn PeerConnectionFactory>>` 注册表 |
| `engine_macros.rs` | `gkit_register_rtc_backend!` 静态初始化宏 |
| `core.rs` | 去掉 `type PC`，`create_peer_connection` 返回 `Box<dyn PeerConnection>` |
| `native/mod.rs` | 保持 `#[cfg(feature)]` 条件编译，移除互斥守卫 |
| `lib.rs` | `make_peer_connection()` → `RtcEngine::create_default()` |

**不变**：
- `PeerConnection` trait 已对象安全（返回 `Box<dyn DataChannel>` 等），不需要改
- C FFI `PcHandleBox.inner: Box<dyn PcTrait>` 不变

---

## 2. Core Traits 改动

当前 `PeerConnectionFactory`（不可做 trait object）：

```rust
pub trait PeerConnectionFactory {
    type PC: PeerConnection;
    fn create_peer_connection(&self) -> MediaResult<Self::PC>;
    fn create_peer_connection_with_config(&self, config: &RtcConfiguration) -> MediaResult<Self::PC>;
}
```

改为对象安全版本：

```rust
pub trait PeerConnectionFactory: Send {
    fn backend_name(&self) -> &'static str;
    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>>;
    fn create_peer_connection_with_config(&self, config: &RtcConfiguration) -> MediaResult<Box<dyn PeerConnection>>;
}
```

| 变更 | 原因 |
|------|------|
| 去掉 `type PC` 关联类型 | 关联类型导致 trait 不可做 trait object |
| 返回 `Box<dyn PeerConnection>` | 运行时多后端必须动态分发 |
| 新增 `backend_name()` | 注册标识名 |
| trait 加 `Send` bound | Factory 存入全局注册表需跨线程安全 |

各后端实现示例：

```rust
// webrtc_rs.rs
impl PeerConnectionFactory for NativeFactory {
    fn backend_name(&self) -> &'static str { "webrtc-rs" }
    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> {
        Ok(Box::new(NativePeerConnection::new()?))
    }
    fn create_peer_connection_with_config(&self, c: &RtcConfiguration) -> MediaResult<Box<dyn PeerConnection>> {
        Ok(Box::new(NativePeerConnection::new()?))
    }
}
```

---

## 3. RtcEngine 注册中心

```rust
// engine.rs
use std::collections::HashMap;
use std::sync::{RwLock, OnceLock};

type FactoryCreator = fn() -> Box<dyn PeerConnectionFactory>;

fn registry() -> &'static RwLock<HashMap<&'static str, FactoryCreator>> {
    static REG: OnceLock<RwLock<HashMap<&'static str, FactoryCreator>>> = OnceLock::new();
    REG.get_or_init(|| RwLock::new(HashMap::new()))
}

pub struct RtcEngine;

impl RtcEngine {
    /// 按名称创建后端工厂
    pub fn create(backend_name: &str) -> MediaResult<Box<dyn PeerConnectionFactory>> {
        let map = registry().read().unwrap();
        let creator = map.get(backend_name)
            .ok_or_else(|| MediaError::new(format!("unknown backend: {backend_name}")))?;
        creator()
    }

    /// 注册后端
    pub fn register(name: &'static str, creator: FactoryCreator) {
        registry().write().unwrap().entry(name).or_insert(creator);
    }

    /// 已注册的后端名称列表
    pub fn registered_types() -> Vec<String> {
        registry().read().unwrap().keys().map(|k| k.to_string()).collect()
    }

    /// 创建默认后端（优先级：webrtc-rs > google_lk > wasm > 第一个）
    pub fn create_default() -> MediaResult<Box<dyn PeerConnectionFactory>>;

    /// 平台自动调度：Jetson(/etc/nv_tegra_release) → google_lk，其余 → webrtc-rs
    pub fn create_for_platform() -> MediaResult<Box<dyn PeerConnectionFactory>>;
}
```

### 静态注册宏

使用 `ctor` crate 实现静态自注册（跨平台可靠，linker section 在 macOS 不可用）：

```rust
// engine_macros.rs — 类似 OpenCTK 的 OCTK_RTC_ENGINE_REGISTER_FACTORY
#[macro_export]
macro_rules! gkit_register_rtc_backend {
    ($name:expr, $factory:ty) => {
        #[doc(hidden)]
        #[cfg_attr(not(test), ::ctor::ctor)]
        fn __gkit_rtc_register() {
            $crate::protocols::rtc::client::engine::RtcEngine::register(
                $name,
                || Box::new(<$factory as Default>::default()),
            );
        }
    };
}
```

`ctor` 作为可选依赖，由 `backend-native` feature 控制（因为 webrtc-rs 和 google 都需要注册）。

### 各后端注册

```rust
// native/webrtc_rs.rs 底部
#[cfg(feature = "backend-native-webrtc-rs")]
gkit_register_rtc_backend!("webrtc-rs", NativeFactory);

// native/google.rs 底部
#[cfg(feature = "backend-native-google")]
gkit_register_rtc_backend!("google_lk", GoogleFactory);

// wasm.rs 底部
#[cfg(feature = "backend-wasm")]
gkit_register_rtc_backend!("wasm", WasmFactory);
```

### lib.rs 工厂函数

```rust
pub fn make_peer_connection() -> Box<dyn PeerConnection> {
    RtcEngine::create_default()
        .expect("no RTC backend registered")
        .create_peer_connection()
        .expect("failed to create PeerConnection")
}

pub fn make_peer_connection_with_backend(name: &str) -> MediaResult<Box<dyn PeerConnection>> {
    RtcEngine::create(name)?.create_peer_connection()
}
```

---

## 4. Feature Flags

```toml
[features]
default = ["backend-native-google"]
backend-native = ["dep:ctor"]
backend-native-all = ["backend-native", "backend-native-webrtc-rs", "backend-native-google"]
backend-native-webrtc-rs = ["backend-native", "dep:webrtc", "dep:tokio", "dep:bytes", "dep:openh264", "dep:rtp"]
backend-native-google = ["backend-native", "dep:cxx", "dep:tokio", "dep:parking_lot",
    "dep:thiserror", "dep:log", "dep:enum_dispatch", "dep:scoped-tls", "dep:futures"]
backend-wasm = []

[dependencies]
ctor = { version = "0.2", optional = true }
```

| 变化 | 说明 |
|------|------|
| **default = "backend-native-google"** | google_lk 优先作为默认后端；webrtc-rs/wasm 默认关闭 |
| 移除 `compile_error!` 互斥守卫 | 两个后端可同时编译 |
| 新增 `backend-native-all` umbrella | 一键启用所有 native |
| google feature 新增依赖 | cxx, parking_lot, thiserror, log, enum_dispatch, scoped-tls, futures |
| 保留单独 feature | `--no-default-features --features backend-native-webrtc-rs` 只编译一个后端 |

native/mod.rs 简化：

```rust
#[cfg(feature = "backend-native-webrtc-rs")]
mod webrtc_rs;
#[cfg(feature = "backend-native-webrtc-rs")]
pub use webrtc_rs::*;

#[cfg(feature = "backend-native-google")]
mod google;
#[cfg(feature = "backend-native-google")]
pub use google::*;

#[cfg(not(any(feature = "backend-native-webrtc-rs", feature = "backend-native-google")))]
compile_error!("at least one native backend feature required");
```

build-sys 解除注释（lib.rs）：

```rust
#[cfg(feature = "backend-native-google")]
#[path = "build-sys/mod.rs"]
pub mod build_sys;
```

---

## 5. C FFI 扩展

### 新增 Factory API

```c
// 创建/销毁工厂
void*  gkit_media_rtc_create_factory(const char* backend_name);
void   gkit_media_rtc_destroy_factory(void* factory);
const char* gkit_media_rtc_factory_backend_name(void* factory);

// 查询已注册后端
int    gkit_media_rtc_get_registered_backends(char*** out_names);
void   gkit_media_rtc_free_string_array(char** arr, int count);

// 基于 Factory 创建 PC
void*  gkit_media_rtc_factory_create_peer_connection(void* factory);
```

### 现有 API 不变

```c
void*  gkit_media_rtc_create_peer_connection();   // 内部改为 RtcEngine::create_default()
// 其余所有函数不变
```

### Rust 侧实现

```rust
struct FactoryHandleBox {
    inner: Box<dyn PeerConnectionFactory>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_rtc_create_factory(
    backend_name: *const c_char,
) -> *mut c_void {
    let name = CStr::from_ptr(backend_name).to_str().unwrap_or_default();
    match RtcEngine::create(name) {
        Ok(f) => Box::into_raw(Box::new(FactoryHandleBox { inner: f })) as *mut c_void,
        Err(_) => ptr::null_mut(),
    }
}
```

---

## 6. C++ RAII 封装

新文件：`apis/cpp/gkit-media/gkit_media_rtc.hpp`

```cpp
namespace gkit {

class RtcFactory {
public:
    static RtcFactory create(const std::string& backendName);
    static RtcFactory createDefault();
    static std::vector<std::string> registeredBackends();

    ~RtcFactory();
    RtcFactory(RtcFactory&&) noexcept;
    RtcFactory& operator=(RtcFactory&&) noexcept;

    PeerConnection createPeerConnection();
    std::string backendName() const;
    bool valid() const;

private:
    void* handle_ = nullptr;
};

class PeerConnection { /* 不变 */ };
class DataChannel     { /* 不变 */ };

} // namespace gkit
```

---

## 7. google_lk 后端激活

### google.rs 重写（替换 stub）

```rust
use crate::protocols::rtc::client::core::*;
use crate::protocols::rtc::client::native::google_lk as lk;

pub struct GooglePeerConnection {
    inner: lk::peer_connection::PeerConnection,
    rt: &'static tokio::runtime::Runtime,
}

pub struct GoogleDataChannel {
    inner: lk::data_channel::DataChannel,
    rt: &'static tokio::runtime::Runtime,
}

pub struct GoogleFactory {
    rt: &'static tokio::runtime::Runtime,
}
```

Async → Sync 桥接：

```rust
impl PeerConnection for GooglePeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        self.rt.block_on(async {
            let desc = self.inner.create_offer().await.map_err(|e| MediaError::new(e))?;
            Ok(SessionDescription { sdp_type: desc.sdp_type, sdp: desc.sdp })
        })
    }
    // ... 其余方法
}
```

枚举映射：

| google_lk | gkit core |
|-----------|-----------|
| `PeerConnectionState` | `ConnectionState` |
| `IceConnectionState` | `IceConnectionState` |
| `IceGatheringState` | `GatheringState` |
| `DataChannelState` | `DataChannelState` |

### google_lk 内部路径修复

google_lk 代码从 LiveKit 移植，内部使用 `crate::imp::*`（指向 native/web 实现）。**29 处** `use webrtc_sys::*` 需要批量替换为 `use crate::build_sys::webrtc_sys::*`（google_lk 假设 `webrtc_sys` 是独立 crate，GenericKit 中它是子模块）：

```bash
# 在 google_lk/native/ 下 27 个文件 + google_lk/mod.rs 中
sed -i 's/use webrtc_sys::/use crate::build_sys::webrtc_sys::/g' \
    crates/gkit-media/src/protocols/rtc/client/native/google_lk/native/*.rs \
    crates/gkit-media/src/protocols/rtc/client/native/google_lk/mod.rs
```

同时确认 `native/peer_connection_factory.rs` 能通过 `crate::build_sys::webrtc_sys` 访问 sys FFI。

### build.rs — LibWebRTC 预编译库下载与缓存

参考 [LiveKit rust-sdks](https://github.com/livekit/rust-sdks) 的打包方式，使用 LiveKit 预编译的 libwebrtc。

**文件**: `crates/gkit-media/build.rs`

**流程**:

```
GKIT_CUSTOM_WEBRTC 设置?
  ├─ 是 → 使用指定路径，跳过下载
  └─ 否 → 检查本地缓存 (.gkit-cache/libwebrtc-<platform>.tar.gz)
              ├─ 存在 → 解压使用
              └─ 不存在 → 下载 LiveKit release → 存入缓存 → 解压
```

**下载源**: `https://github.com/livekit/rust-sdks/releases/download/livekit-ffi-v0.14.0/libwebrtc-<platform>.tar.gz`

**平台映射**:

| 目标 | 文件 tag |
|------|---------|
| `x86_64-unknown-linux-gnu` | `linux-x86_64` |
| `aarch64-unknown-linux-gnu` | `linux-aarch64` |
| `x86_64-apple-darwin` | `macos-x86_64` |
| `aarch64-apple-darwin` | `macos-aarch64` |
| `x86_64-pc-windows-msvc` | `windows-x86_64` |

**环境变量**:

| 变量 | 说明 |
|------|------|
| `GKIT_CUSTOM_WEBRTC` | 自定义 libwebrtc 路径（跳过下载） |
| `GKIT_SKIP_WEBRTC_DOWNLOAD` | 跳过下载，仅使用缓存 |
| `GKIT_WEBRTC_CACHE_DIR` | 缓存目录（默认 `.gkit-cache`） |

**编译步骤**（在解压后）:

```rust
// build.rs (仅 backend-native-google 时执行)
fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let lib_dir = download_or_cache_libwebrtc(manifest_dir);

    // 编译 webrtc-sys C++ 源文件
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include("src/build-sys/webrtc-sys/include")
        .include(format!("{lib_dir}/include"))
        .files(webrtc_sys_cpp_files())
        .compile("gkit_webrtc_sys");

    // 链接 libwebrtc
    println!("cargo:rustc-link-search=native={lib_dir}/lib");
    println!("cargo:rustc-link-lib=static=webrtc");

    // 平台特定链接
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=pthread");
    }
}
```

---

## 8. Rust 设计决策与注意事项

设计中有多处与 C++ 习惯不同的 Rust 特化选择，以下逐一标注。

### 8.1 静态注册：`ctor` vs `inventory` vs 手动 init

当前选择 `ctor` crate（≈ C++ 的 `__attribute__((constructor))`）。

| 方案 | 优点 | 缺点 |
|------|------|------|
| `ctor` (当前) | 简单直接，与 OpenCTK 的 static global constructor 等价 | 依赖运行时构造函数，在 `cdylib` 中可能不被调用；某些嵌入式平台不支持 |
| `inventory` crate | 纯链接期注册，无运行时开销；比 ctor 更「Rust 味」 | 需要 `inventory` + `submit` 两个 crate，API 略繁琐 |
| 手动 `RtcEngine::init()` | 最可控，无外部依赖 | 用户忘记调用则注册表为空 |

**建议**：先用 `ctor` 快速落地，后续如遇跨平台问题再切 `inventory`。两者对后端代码（宏调用方）透明。

### 8.2 `Mutex<HashMap>` vs `RwLock<HashMap>`

当前用 `Mutex<HashMap>` 保护全局注册表。注册表的特点是：**写极少（启动时各后端注册一次），读频繁（每次创建 PC 都查表）**。

```rust
// 当前
fn registry() -> &'static Mutex<HashMap<&'static str, FactoryCreator>> { ... }

// 建议改为 RwLock（或用 OnceLock 彻底消除锁）
fn registry() -> &'static RwLock<HashMap<&'static str, FactoryCreator>> { ... }
```

**更激进方案**：用 `OnceLock<HashMap>` + `register()` 内部拿锁重建整个 HashMap（因为只在启动时写一次），`create()` 无锁读。复杂度较高，先 `RwLock` 即可。

### 8.3 `Box<dyn Fn()>` closure vs `fn()` 函数指针

```rust
// 当前：用闭包装箱（灵活但多一层间接调用）
type FactoryCreator = Box<dyn Fn() -> Box<dyn PeerConnectionFactory> + Send + Sync>;

// 备选：用裸函数指针（零开销，但无法捕获状态）
type FactoryCreator = fn() -> Box<dyn PeerConnectionFactory>;
```

如果 Factory 创建不需要捕获任何状态（目前设计确实不需要），`fn()` 指针更高效且无需堆分配。**建议改为 `fn()`**。

### 8.4 Tokio Runtime 生命周期

```rust
pub struct GoogleFactory {
    rt: &'static tokio::runtime::Runtime,  // 持有 static 引用
}
```

`OnceLock<Runtime>` 初始化的 runtime **永远不会被 drop**，进程退出时线程可能未正确 join。对短期程序无影响，长期运行的服务可考虑：
- 注册一个 `drop` handler
- 或使用 `tokio::runtime::Handle` 替代 `&Runtime`（Handle 更轻量，可 clone）

**当前影响**：非关键，仅在生产环境长期运行时需关注。

### 8.5 Trait object 与 Send 约束

```rust
pub trait PeerConnectionFactory: Send { ... }
```

`PeerConnection` trait 本身也需要 `Send`（C FFI 的 `PcHandleBox.inner: Box<dyn PcTrait>` 已隐式要求）。当前 `PeerConnection` trait 定义中：
- 方法参数有 `Box<dyn VideoSink>` — 这需要 `VideoSink: Send` 才能整体 `Send`

确认 `VideoSink` / `VideoSource` 已有 `Send` bound（`source_sink.rs` 中应已有），否则 trait object 组合会编译报错。

### 8.6 `#[cfg_attr(not(test), ::ctor::ctor)]` — 测试时行为

`not(test)` 条件确保 `#[test]` 编译时不会触发 `ctor`，因为 `cargo test` 把主代码和测试编译在一起，`#[ctor]` 会重复注册。但这也意味着**测试中需要手动调用注册**：

```rust
#[test]
fn test_engine() {
    // 测试中 ctor 不触发，需手动注册
    RtcEngine::register("webrtc-rs", Box::new(|| Box::new(NativeFactory::default())));
    // ...
}
```

或者改为在测试模块中显式 import 注册代码路径（而非依赖 `ctor`）。

---

## 9. 测试策略

### 当前默认

- **默认后端**: `google_lk`（`backend-native-google`）
- **webrtc-rs/wasm**: 默认关闭，按需 `--features` 启用
- 所有测试通过 `RtcEngine::create_default()` 创建后端

### 受影响需修改的现有代码

| 文件 | 处理 |
|------|------|
| `tests/webrtc_*.rs` | `NativeFactory::default()` → `RtcEngine::create_default()?` |
| `examples/gkit-media-webrtc-loopback/main.rs` | 改为 RtcEngine 创建（使用 `gkit_media::make_peer_connection()` 或 factory） |
| C 测试 | 现有 API 兼容，无需改动（内部调用 `make_peer_connection()`） |

### 新增测试

| 层 | 测试 | 命令 |
|---|---|---|
| Rust — P2P 连通性 | `p2p_ice_connectivity` — 两个 PC SDP/ICE 交换，验证连接状态 | `cargo test -p gkit-media --test webrtc_p2p_conn -- --nocapture` |
| Rust — 引擎注册 | `rtc_engine_register`, `rtc_engine_create`, `rtc_engine_default` | `cargo test -p gkit-media` |
| Rust — 多后端 | 同一测试中分别创建 google_lk 和 webrtc-rs PC | `cargo test --features backend-native-all` |
| C FFI | `test_rtc_factory.c` | `ctest -R gkit_media_c_test` |
| C++ GTest | `test_rtc_factory.cpp` | `ctest -R gkit_media_cpp_test_rtc` |

### 验证命令

```bash
cargo test -p gkit-media                                         # default (all backends)
cargo test -p gkit-media --features backend-native-all           # explicit all
cargo test -p gkit-media --no-default-features --features backend-native-webrtc-rs  # single
cargo check -p gkit-media --features backend-native-google       # google compile
ctest --test-dir build-auto -R gkit_media                        # C/C++ tests
cargo test -p gkit-media && ctest --test-dir build-auto          # full verify
```

---

## 10. Non-Goals

- 不改 `PeerConnection` trait — 已对象安全
- 不移除现有 feature flag 体系 — 保留裁剪能力
- 不修改 google_lk 内部代码（只修导入路径）
- E2EE (FrameCryptor)、Desktop capture、Stats、RTP transceiver — 不在范围

---

## 11. Phase Summary

| Phase | 描述 | 状态 |
|-------|------|------|
| 1 | core traits 改动 + engine.rs + 宏 | ✅ 已实现 |
| 2 | webrtc-rs 适配新 trait | ✅ 已实现（默认关闭） |
| 3 | google.rs 重写 + google_lk 激活 | ✅ 已实现 |
| 4 | Feature flags + mod.rs + Cargo.toml | ✅ 已实现 |
| 5 | build.rs — LibWebRTC 下载/缓存/编译 | ✅ 已创建 |
| 6 | C FFI 新 API + C++ RtcFactory 封装 | ⏳ 待实现 |
| 7 | 现有测试/示例迁移 | ✅ 已迁移 |

## 12. Implementation Status (2026-05-07)

### 已完成

| 文件 | 变更 |
|------|------|
| `core.rs` | `PeerConnectionFactory` 去掉关联类型，返回 `Box<dyn PeerConnection>` |
| `engine.rs` | `RtcEngine` 全局注册中心（`fn()` 指针, `RwLock`） |
| `engine_macros.rs` | `gkit_register_rtc_backend!` 宏（`ctor` crate） |
| `google.rs` | 完整重写：`GooglePeerConnection`/`GoogleDataChannel`/`GoogleFactory` 包装 google_lk |
| `webrtc_rs.rs` | 适配新 trait + 注册宏 |
| `wasm.rs` | 适配新 trait + 注册宏 |
| `native/mod.rs` | 移除互斥编译守卫 |
| `lib.rs` | 解除 `build_sys` 注释；`make_peer_connection()` 改为调用 `RtcEngine` |
| `Cargo.toml` | default → `backend-native-google`；新增 google 依赖；新增 umbrella feature |
| `build.rs` | LibWebRTC 下载/缓存/编译脚本（LiveKit 预编译 release） |
| `tests/*.rs` | `NativeFactory` → `RtcEngine::create_default()` |
| `example/*loopback*` | 引擎模式 |
| `google_lk/native/*.rs` | 29 处 `use webrtc_sys::` → `use crate::build_sys::webrtc_sys::` |

### 待编译验证

由于 cargo 依赖下载超时（crates.io 网络），尚未完成 `cargo check` 编译验证。

**编译命令**（有网络时执行）：

```bash
# 默认 google_lk 后端
cargo check -p gkit-media

# 或跳过 LibWebRTC 下载（仅检查 Rust 代码）
GKIT_SKIP_WEBRTC_DOWNLOAD=true cargo check -p gkit-media

# 全后端
cargo check -p gkit-media --features backend-native-all
```

**已知风险**：
- `ctor` crate 版本为 0.2，需确认没有破坏性变更
- google_lk 的编译依赖 libwebrtc C++ 二进制，需要 LiveKit release 可访问
- `build.rs` 中 `std("c++17")` 需要系统安装 C++17 编译器
- 平台 link 依赖（X11, dl, pthread）仅 Linux 验证过
