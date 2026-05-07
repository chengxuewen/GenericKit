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
use std::sync::{Mutex, OnceLock};

type FactoryCreator = Box<dyn Fn() -> Box<dyn PeerConnectionFactory> + Send + Sync>;

fn registry() -> &'static Mutex<HashMap<&'static str, FactoryCreator>> {
    static REG: OnceLock<Mutex<HashMap<&'static str, FactoryCreator>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct RtcEngine;

impl RtcEngine {
    /// 按名称创建后端工厂
    pub fn create(backend_name: &str) -> MediaResult<Box<dyn PeerConnectionFactory>>;

    /// 注册后端（每个后端在模块初始化时调用）
    pub fn register(name: &'static str, creator: FactoryCreator);

    /// 已注册的后端名称列表
    pub fn registered_types() -> Vec<String>;

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
                Box::new(|| Box::new(<$factory as Default>::default())),
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
default = ["backend-native-all"]

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
| 移除 `compile_error!` 互斥守卫 | 两个后端可同时编译 |
| 新增 `backend-native-all` umbrella | default 一键启用所有 native |
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

---

## 8. 测试策略

### 受影响需修改的现有代码

| 文件 | 处理 |
|------|------|
| `tests/webrtc_p2p.rs`, `webrtc_track.rs`, `webrtc_ice.rs`, `webrtc_p2p_conn.rs` | `NativePeerConnection::new()` → `RtcEngine::create("webrtc-rs")?.create_peer_connection()?` |
| `examples/gkit-media-webrtc-loopback/main.rs` | 同样改为 RtcEngine 创建 |
| C 测试 `test_basic.c` 等 | 现有 API 兼容，无需改动 |

### 新增测试

| 层 | 测试 | 命令 |
|---|---|---|
| Rust — 注册/创建 | `rtc_engine_register`, `rtc_engine_create`, `rtc_engine_default`, `rtc_engine_platform` | `cargo test -p gkit-media` |
| Rust — 多后端 | 同一测试中分别创建 webrtc-rs 和 google_lk PC | `cargo test --features backend-native-all` |
| Rust — 裁剪 | 只编译 webrtc-rs，验证注册表 | `cargo test --no-default-features --features backend-native-webrtc-rs` |
| C FFI | `test_rtc_factory.c` — factory 生命周期、注册表 | `ctest -R gkit_media_c_test` |
| C++ GTest | `test_rtc_factory.cpp` — RtcFactory RAII | `ctest -R gkit_media_cpp_test_rtc` |

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

## 9. Non-Goals

- 不改 `PeerConnection` trait — 已对象安全
- 不移除现有 feature flag 体系 — 保留裁剪能力
- 不修改 google_lk 内部代码（只修导入路径）
- E2EE (FrameCryptor)、Desktop capture、Stats、RTP transceiver — 不在范围

---

## 10. Phase Summary

| Phase | 描述 | 依赖 |
|-------|------|------|
| 1 | core traits 改动 + engine.rs + 宏 | — |
| 2 | webrtc-rs 适配新 trait | Phase 1 |
| 3 | google.rs 重写 + google_lk 激活 | Phase 1, libwebrtc binary |
| 4 | Feature flags 调整 + mod.rs 修改 | Phase 1 |
| 5 | C FFI 新 API + C++ RtcFactory 封装 | Phase 1 |
| 6 | 现有测试/示例迁移 + 新测试 | Phase 1-5 |
