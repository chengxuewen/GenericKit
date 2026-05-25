# livekit-rs Backend Migration 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `gkit-media` 的 Google libwebrtc 后端从自维护 CXX FFI 胶水代码迁移为外部依赖 LiveKit rust-sdks 的 `libwebrtc` crate，删除 ~123 个胶水文件，新写 ~10 个 adapter 模块。

**Architecture:** 删除 `build-sys/webrtc-sys/` + `google_lk/`，新建 `livekit_rs/` adapter 模块。`core.rs` trait 不变，通过 `From`/`Into` 做类型转换。RtcEngine 注册机制不变。

**Tech Stack:** Rust edition 2024, `libwebrtc` (LiveKit rust-sdks git dep), `yuv-sys` (LiveKit), tokio, Cargo feature flags, GTest/CTest unchanged.

**Spec Reference:** `docs/superpowers/specs/2026-05-25-livekit-rs-backend-migration-design.md`

---

### Task 1: 验证依赖共用情况

**Files:**
- Check: `crates/gkit-media/Cargo.toml`

- [ ] **Step 1: 逐项检查要删除的依赖是否被 webrtc-rs 共用**

```bash
# 检查每个依赖的引用来源
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert cxx 2>/dev/null || echo "cxx: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert parking_lot 2>/dev/null || echo "parking_lot: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert thiserror 2>/dev/null || echo "thiserror: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert serde 2>/dev/null || echo "serde: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert serde_json 2>/dev/null || echo "serde_json: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert log 2>/dev/null || echo "log: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert lazy_static 2>/dev/null || echo "lazy_static: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert futures 2>/dev/null || echo "futures: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert rtrb 2>/dev/null || echo "rtrb: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert enum_dispatch 2>/dev/null || echo "enum_dispatch: NOT shared"
cargo tree -p gkit-media --features backend-native-webrtc-rs -e features --invert scoped-tls 2>/dev/null || echo "scoped-tls: NOT shared"
```

请在终端执行以上命令并记录结果。如果某依赖被共用（输出不只是 "NOT shared"），则**不可删**，需保留在默认依赖或 webrtc-rs 的 feature 中。

- [ ] **Step 2: 记录核查结论**

```
共用依赖清单（不可删）:
[从 Step 1 输出中提取]

独享依赖清单（可删除）:
[标记为 NOT shared 的所有依赖]
```

---

### Task 2: 删除旧代码目录

**Files:**
- Delete: `crates/gkit-media/src/build-sys/webrtc-sys/` (全部 123 文件)
- Delete: `crates/gkit-media/src/build-sys/yuv-sys/`
- Delete: `crates/gkit-media/src/protocols/rtc/client/native/google_lk/` (全部 25 模块)
- Delete: `crates/gkit-media/src/protocols/rtc/client/native/google.rs`
- Modify: `crates/gkit-media/src/build-sys/mod.rs`

- [ ] **Step 1: 删除 webrtc-sys 和 yuv-sys**

```bash
rm -rf crates/gkit-media/src/build-sys/webrtc-sys/
rm -rf crates/gkit-media/src/build-sys/yuv-sys/
```

- [ ] **Step 2: 删除 google_lk 后端和 google.rs adapter**

```bash
rm -rf crates/gkit-media/src/protocols/rtc/client/native/google_lk/
rm crates/gkit-media/src/protocols/rtc/client/native/google.rs
```

- [ ] **Step 3: 更新 build-sys/mod.rs，删除 webrtc_sys 引用**

读取 `crates/gkit-media/src/build-sys/mod.rs`，删除以下行：

```rust
#[cfg(feature = "backend-native-google")]
#[path = "webrtc-sys/lib.rs"]
pub mod webrtc_sys;
```

如果 mod.rs 只剩下空行，删除整个 mod.rs 文件。

- [ ] **Step 4: 更新 lib.rs，删除 build_sys feature gate**

读取 `crates/gkit-media/src/lib.rs`，删除以下 block：

```rust
// 移除 build_sys 相关的 #[cfg(feature = "backend-native-google")]
pub mod build_sys;
```

如果 mod.rs 保留但为空，可以保留。如果完全没有内容，删除该模块声明。

---

### Task 3: 更新 Cargo.toml 依赖

**Files:**
- Modify: `crates/gkit-media/Cargo.toml`

- [ ] **Step 1: 删除 Task 1 确认的独享依赖**

在 `[dependencies]` 部分删除 Task 1 标记为"可删除"的依赖行。例如：

```toml
# 删除以下行（如果 Task 1 确认无人共享）：
cxx = { workspace = true, optional = true }
parking_lot = { workspace = true, optional = true }
lazy_static = { workspace = true, optional = true }
rtrb = { version = "0.3", optional = true }
enum_dispatch = { version = "0.3", optional = true }
scoped-tls = { version = "1", optional = true }
```

- [ ] **Step 2: 保留被共用的依赖但不再作为 google feature 的 gate**

如果 Task 1 确认 `serde`, `serde_json`, `log` 等被 webrtc-rs 共用，将它们从 `backend-native-google` 的 feature 列表中移除（不删除依赖本身），改放 `backend-native` 或作为直接依赖。

- [ ] **Step 3: 添加 LiveKit 依赖**

在 `[dependencies]` 部分添加：

```toml
libwebrtc = { git = "https://github.com/livekit/rust-sdks", tag = "libwebrtc-v0.3.34", optional = true }
yuv-sys = { git = "https://github.com/livekit/rust-sdks", tag = "yuv-sys-v0.3.14", optional = true }
```

- [ ] **Step 4: 更新 feature 列表**

```toml
[features]
default = ["backend-native"]
backend-native = []
backend-native-webrtc-rs = ["backend-native", "dep:webrtc", "dep:tokio"]  # 保持原样
backend-native-google = ["backend-native", "dep:libwebrtc", "dep:yuv-sys"]  # 简化
backend-native-all = ["backend-native-webrtc-rs", "backend-native-google"]
backend-wasm = []
```

删除 `backend-native-google` 原有的 14 个依赖列表。

- [ ] **Step 5: 验证 Cargo.toml 语法**

```bash
cargo verify-project 2>&1 || true
cargo metadata --no-deps 2>&1 | head -5
```

确认无 JSON 解析错误。

---

### Task 4: 更新 native/mod.rs 后端注册

**Files:**
- Modify: `crates/gkit-media/src/protocols/rtc/client/native/mod.rs`

- [ ] **Step 1: 替换 google_lk/goggle 引用为 livekit_rs**

读取 `crates/gkit-media/src/protocols/rtc/client/native/mod.rs`，找到以下 block：

```rust
#[cfg(feature = "backend-native-google")]
mod google;
#[cfg(feature = "backend-native-google")]
mod google_lk;
#[cfg(feature = "backend-native-google")]
gkit_register_rtc_backend!(google::GoogleFactory, "google");
```

替换为：

```rust
#[cfg(feature = "backend-native-google")]
mod livekit_rs;
#[cfg(feature = "backend-native-google")]
gkit_register_rtc_backend!(livekit_rs::LiveKitRsFactory, "google");
```

---

### Task 5: 编译验证

- [ ] **Step 1: Rust 检查**

```bash
cargo check -p gkit-media --features backend-native-google 2>&1
```

预期：大量编译错误（`livekit_rs` 模块尚不存在），但 `build-sys/` 和 `google_lk/` 相关的路径错误应该消失了。如果仍有 `webrtc_sys` 或 `google_lk` 的引用错误，说明有遗漏的引用，需 grep 修复。

- [ ] **Step 2: 搜索残留引用**

```bash
grep -rn "google_lk\|google.rs\|webrtc_sys\|webrtc-sys" crates/gkit-media/src/ 2>/dev/null
```

预期：无输出。如有输出，逐项修复或删除。

```bash
grep -rn "build_sys\|build-sys" crates/gkit-media/src/lib.rs 2>/dev/null
```

预期：无输出（已在 Task 2 Step 4 清理）。如有残留，删除。

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "refactor: delete google_lk and webrtc-sys glue code, add libwebrtc dep"
```

---

### Task 6: 创建 livekit_rs/ 骨架

**Files:**
- Create: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/mod.rs`
- Create: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/factory.rs`

- [ ] **Step 1: 创建 mod.rs — 模块声明 + RtcEngine 注册**

```rust
// crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/mod.rs

use crate::protocols::rtc::client::core::PeerConnectionFactory;

mod factory;
pub use factory::LiveKitRsFactory;

// 实现 RtcEngine 需要的注册函数
pub fn create_factory() -> Box<dyn PeerConnectionFactory> {
    Box::new(LiveKitRsFactory::new())
}

// 注册宏需要 pub 可见的 struct
pub struct LiveKitRsBackend;
```

- [ ] **Step 2: 创建 factory.rs — 最小实现（TODO 阶段，仅编译通过）**

```rust
// crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/factory.rs

use crate::protocols::rtc::client::core::{PeerConnection, PeerConnectionFactory, RtcConfiguration};
use crate::media::MediaResult;

pub struct LiveKitRsFactory;

impl LiveKitRsFactory {
    pub fn new() -> Self {
        Self
    }
}

impl PeerConnectionFactory for LiveKitRsFactory {
    fn backend_name(&self) -> &'static str {
        "google"
    }

    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> {
        todo!("implement with libwebrtc")
    }

    fn create_peer_connection_with_config(
        &self,
        _config: &RtcConfiguration,
    ) -> MediaResult<Box<dyn PeerConnection>> {
        todo!("implement with libwebrtc")
    }
}
```

- [ ] **Step 3: 更新 native/mod.rs 注册宏调用**

找到 Task 4 中添加的注册宏行，更新为：

```rust
gkit_register_rtc_backend!(livekit_rs::LiveKitRsBackend, "google");
```

- [ ] **Step 4: 编译检查**

```bash
cargo check -p gkit-media --features backend-native-google 2>&1
```

预期：编译通过（`todo!()` 可编译，但运行时 panic）。

- [ ] **Step 5: 提交**

```bash
git add crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/
git add crates/gkit-media/src/protocols/rtc/client/native/mod.rs
git commit -m "feat: add livekit_rs backend skeleton with stubbed factory"
```

---

### Task 7: 实现 PeerConnection adapter (L0 + L1)

**Files:**
- Create: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/peer_connection.rs`

- [ ] **Step 1: 写 L0 单测 — 验证 observer 回调类型映射**

```rust
// crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/peer_connection.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_state_mapping() {
        // 验证 libwebrtc::PeerConnectionState → 我们的 ConnectionState
        use libwebrtc::native::peer_connection::PeerConnectionState as LkState;
        use crate::protocols::rtc::client::core::ConnectionState as OurState;

        let cases = vec![
            (LkState::New, OurState::New),
            (LkState::Connecting, OurState::Connecting),
            (LkState::Connected, OurState::Connected),
            (LkState::Disconnected, OurState::Disconnected),
            (LkState::Failed, OurState::Failed),
            (LkState::Closed, OurState::Closed),
        ];

        for (lk, expected) in cases {
            assert_eq!(OurState::from(lk), expected,
                "state {:?} mapped incorrectly", lk);
        }
    }

    #[test]
    fn ice_connection_state_mapping() {
        use libwebrtc::native::peer_connection::IceConnectionState as LkState;
        use crate::protocols::rtc::client::core::IceConnectionState as OurState;

        assert_eq!(OurState::from(LkState::New), OurState::New);
        assert_eq!(OurState::from(LkState::Checking), OurState::Checking);
        assert_eq!(OurState::from(LkState::Connected), OurState::Connected);
        assert_eq!(OurState::from(LkState::Completed), OurState::Connected);
        assert_eq!(OurState::from(LkState::Failed), OurState::Failed);
        assert_eq!(OurState::from(LkState::Disconnected), OurState::Disconnected);
        assert_eq!(OurState::from(LkState::Closed), OurState::Closed);
    }
}
```

- [ ] **Step 2: 跑单测 — 预期 FAIL（From 未实现）**

```bash
cargo test -p gkit-media --lib --features backend-native-google -- connection_state 2>&1
```

预期：编译失败（`From<LkState> for OurState` 未实现）。

- [ ] **Step 3: 实现 From 转换 + `LiveKitPeerConnection` 结构体**

在同一个文件 `peer_connection.rs` 中，在测试模块**之前**添加：

```rust
use std::sync::mpsc;
use libwebrtc::native::peer_connection as lk;
use crate::protocols::rtc::client::core::{
    PeerConnection, ConnectionState, IceConnectionState, GatheringState,
    SignalingState, SessionDescription, IceCandidate, RtcConfiguration,
};
use crate::media::MediaResult;

// ---- 状态枚举 From 转换 ----

impl From<lk::PeerConnectionState> for ConnectionState {
    fn from(s: lk::PeerConnectionState) -> Self {
        match s {
            lk::PeerConnectionState::New => ConnectionState::New,
            lk::PeerConnectionState::Connecting => ConnectionState::Connecting,
            lk::PeerConnectionState::Connected => ConnectionState::Connected,
            lk::PeerConnectionState::Disconnected => ConnectionState::Disconnected,
            lk::PeerConnectionState::Failed => ConnectionState::Failed,
            lk::PeerConnectionState::Closed => ConnectionState::Closed,
        }
    }
}

impl From<lk::IceConnectionState> for IceConnectionState {
    fn from(s: lk::IceConnectionState) -> Self {
        match s {
            lk::IceConnectionState::New => IceConnectionState::New,
            lk::IceConnectionState::Checking => IceConnectionState::Checking,
            lk::IceConnectionState::Connected => IceConnectionState::Connected,
            lk::IceConnectionState::Completed => IceConnectionState::Connected,
            lk::IceConnectionState::Failed => IceConnectionState::Failed,
            lk::IceConnectionState::Disconnected => IceConnectionState::Disconnected,
            lk::IceConnectionState::Closed => IceConnectionState::Closed,
        }
    }
}

// ---- LiveKitPeerConnection ----

pub struct LiveKitPeerConnection {
    inner: lk::PeerConnection,
}

impl LiveKitPeerConnection {
    pub fn new(pc: lk::PeerConnection) -> Self {
        Self { inner: pc }
    }
}

impl PeerConnection for LiveKitPeerConnection {
    fn connection_state(&self) -> ConnectionState {
        self.inner.connection_state().into()
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        self.inner.ice_connection_state().into()
    }

    // ---- 待 P3 实现的方法 ----
    fn create_offer(&self) -> MediaResult<SessionDescription> { todo!() }
    fn create_answer(&self) -> MediaResult<SessionDescription> { todo!() }
    fn set_local_description(&self, _sd: SessionDescription) -> MediaResult<()> { todo!() }
    fn set_remote_description(&self, _sd: SessionDescription) -> MediaResult<()> { todo!() }
    fn add_ice_candidate(&self, _c: IceCandidate) -> MediaResult<()> { todo!() }
    fn create_data_channel(&self, _label: &str, _init: crate::protocols::rtc::client::core::DataChannelInit) -> MediaResult<Box<dyn super::super::super::core::DataChannel>> { todo!() }
    fn add_track(&self, _track: Box<dyn super::super::super::core::VideoTrack>) -> MediaResult<()> { todo!() }
    fn remove_track(&self, _track: Box<dyn super::super::super::core::VideoTrack>) -> MediaResult<()> { todo!() }
    fn close(&self) -> MediaResult<()> { todo!() }
    fn set_configuration(&self, _config: RtcConfiguration) -> MediaResult<()> { todo!() }
    fn gathering_state(&self) -> GatheringState { todo!() }
    fn signaling_state(&self) -> SignalingState { todo!() }
    fn current_local_description(&self) -> Option<SessionDescription> { todo!() }
    fn current_remote_description(&self) -> Option<SessionDescription> { todo!() }
    fn senders(&self) -> Vec<Box<dyn super::super::super::core::RtpSender>> { todo!() }
    fn receivers(&self) -> Vec<Box<dyn super::super::super::core::RtpReceiver>> { todo!() }
    fn get_stats(&self) -> MediaResult<String> { todo!() }
    fn restart_ice(&self) -> MediaResult<()> { todo!() }
}
```

- [ ] **Step 4: 跑单测 — 预期 PASS**

```bash
cargo test -p gkit-media --lib --features backend-native-google -- connection_state 2>&1
```

预期：2 测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/peer_connection.rs
git commit -m "feat: livekit_rs PeerConnection adapter with state From impls"
```

---

### Task 8: 实现 Factory + PC 创建 (L1 测试 PC 生命周期)

**Files:**
- Modify: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/factory.rs`
- Modify: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/peer_connection.rs`
- Create: `crates/gkit-media/tests/webrtc_lk_basic.rs`

- [ ] **Step 1: 实现 factory.rs 的真实 PC 创建**

重写 `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/factory.rs`：

```rust
use libwebrtc::native::peer_connection_factory as lk_pcf;
use libwebrtc::native::peer_connection as lk_pc;
use crate::protocols::rtc::client::core::{PeerConnection, PeerConnectionFactory, RtcConfiguration};
use crate::media::{MediaResult, MediaError};
use super::peer_connection::LiveKitPeerConnection;
use std::sync::OnceLock;

static PCF: OnceLock<lk_pcf::PeerConnectionFactory> = OnceLock::new();

fn get_pcf() -> &'static lk_pcf::PeerConnectionFactory {
    PCF.get_or_init(|| {
        lk_pcf::PeerConnectionFactory::new()
    })
}

pub struct LiveKitRsFactory;

impl LiveKitRsFactory {
    pub fn new() -> Self { Self }
}

impl PeerConnectionFactory for LiveKitRsFactory {
    fn backend_name(&self) -> &'static str { "google" }

    fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> {
        let config = lk_pc::RtcConfiguration::new();
        self.create_peer_connection_with_config(&RtcConfiguration::default())
    }

    fn create_peer_connection_with_config(
        &self,
        config: &RtcConfiguration,
    ) -> MediaResult<Box<dyn PeerConnection>> {
        let lk_config = lk_pc::RtcConfiguration {
            ice_servers: config.ice_servers.iter().map(|s| lk_pc::IceServer {
                urls: s.urls.clone(),
                username: s.username.clone().unwrap_or_default(),
                password: s.credential.clone().unwrap_or_default(),
            }).collect(),
            ..lk_pc::RtcConfiguration::new()
        };

        let pc = get_pcf()
            .create_peer_connection(lk_config)
            .map_err(|e| MediaError::new(format!("failed to create PC: {e}")))?;

        Ok(Box::new(LiveKitPeerConnection::new(pc)))
    }
}
```

- [ ] **Step 2: 写 L1 集成测试 — PC 生命周期**

创建 `crates/gkit-media/tests/webrtc_lk_basic.rs`：

```rust
#[cfg(feature = "backend-native-google")]
mod lk_tests {
    use gkit_media::protocols::rtc::client::core::{
        PeerConnection, PeerConnectionFactory, RtcConfiguration, ConnectionState,
    };
    use gkit_media::protocols::rtc::client::RtcEngine;

    #[tokio::test]
    async fn create_and_close_peer_connection() {
        let factory = RtcEngine::create("google").expect("google backend not registered");

        let pc = factory.create_peer_connection()
            .expect("failed to create peer connection");

        let state = pc.connection_state();
        assert!(matches!(state, ConnectionState::New | ConnectionState::Connecting),
            "expected New or Connecting, got {:?}", state);

        pc.close().expect("failed to close peer connection");

        let state = pc.connection_state();
        assert_eq!(state, ConnectionState::Closed);
    }

    #[tokio::test]
    async fn create_peer_connection_with_config() {
        let factory = RtcEngine::create("google").expect("google backend not registered");

        let config = RtcConfiguration::default();
        let pc = factory.create_peer_connection_with_config(&config)
            .expect("failed to create PC with config");

        assert!(matches!(pc.connection_state(),
            ConnectionState::New | ConnectionState::Connecting));

        pc.close().unwrap();
    }
}
```

- [ ] **Step 3: 跑集成测试**

```bash
cargo test --features backend-native-google webrtc_lk_basic -- --nocapture 2>&1
```

预期：2 测试 PASS（PC 创建 + 关闭成功）。

- [ ] **Step 4: 提交**

```bash
git add crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/factory.rs
git add crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/peer_connection.rs
git add crates/gkit-media/tests/webrtc_lk_basic.rs
git commit -m "feat: livekit_rs factory with real PC creation + lifecycle test"
```

---

### Task 9: 实现 SDP + ICE adapter (L1 P2P 测试)

**Files:**
- Create: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/session_description.rs`
- Create: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/ice.rs`
- Create: `crates/gkit-media/tests/common/mod.rs`
- Create: `crates/gkit-media/tests/webrtc_lk_p2p.rs`
- Modify: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/mod.rs`

- [ ] **Step 1: 写 L0 单测 — SDP 文本往返**

创建 `livekit_rs/session_description.rs`：

```rust
use libwebrtc::native::jsep as lk_jsep;
use crate::protocols::rtc::client::core::{SessionDescription, SdpType};

impl From<lk_jsep::SessionDescription> for SessionDescription {
    fn from(sd: lk_jsep::SessionDescription) -> Self {
        let sdp_type = match sd.sdp_type() {
            lk_jsep::SdpType::Offer => SdpType::Offer,
            lk_jsep::SdpType::Answer => SdpType::Answer,
            lk_jsep::SdpType::PrAnswer => SdpType::PrAnswer,
        };
        SessionDescription::new(sdp_type, sd.stringify())
    }
}

impl TryFrom<SessionDescription> for lk_jsep::SessionDescription {
    type Error = String;
    fn try_from(sd: SessionDescription) -> Result<Self, Self::Error> {
        lk_jsep::SessionDescription::create(
            sd.sdp_type.into(),
            &sd.sdp,
        ).map_err(|e| format!("invalid SDP: {e}"))
    }
}

impl From<SdpType> for lk_jsep::SdpType {
    fn from(t: SdpType) -> Self {
        match t {
            SdpType::Offer => lk_jsep::SdpType::Offer,
            SdpType::Answer => lk_jsep::SdpType::Answer,
            SdpType::PrAnswer => lk_jsep::SdpType::PrAnswer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_offer_sdp() {
        let sdp_text = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n";
        let lk_sd = lk_jsep::SessionDescription::create(lk_jsep::SdpType::Offer, sdp_text)
            .expect("create SDP");
        let ours: SessionDescription = lk_sd.into();
        assert_eq!(ours.sdp_type, SdpType::Offer);
        assert!(ours.sdp.contains("v=0"));

        let back: lk_jsep::SessionDescription = ours.try_into().expect("back to lk");
        assert_eq!(back.sdp_type(), lk_jsep::SdpType::Offer);
    }

    #[test]
    fn roundtrip_answer_sdp() {
        let sdp_text = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n";
        let lk_sd = lk_jsep::SessionDescription::create(lk_jsep::SdpType::Answer, sdp_text)
            .expect("create SDP");
        let ours: SessionDescription = lk_sd.into();
        assert_eq!(ours.sdp_type, SdpType::Answer);
        let back: lk_jsep::SessionDescription = ours.try_into().expect("back to lk");
        assert_eq!(back.sdp_type(), lk_jsep::SdpType::Answer);
    }
}
```

- [ ] **Step 2: 跑 L0 SDP 单测 — 预期 FAIL 然后实现**

```bash
cargo test -p gkit-media --lib --features backend-native-google -- roundtrip 2>&1
```

- [ ] **Step 3: 实现 ice.rs — IceCandidate 转换**

创建 `livekit_rs/ice.rs`：

```rust
use libwebrtc::native::candidate as lk_candidate;
use crate::protocols::rtc::client::core::IceCandidate;

impl From<lk_candidate::IceCandidate> for IceCandidate {
    fn from(c: lk_candidate::IceCandidate) -> Self {
        IceCandidate {
            sdp_mid: c.sdp_mid(),
            sdp_mline_index: c.sdp_mline_index(),
            candidate: c.stringify(),
        }
    }
}

impl TryFrom<IceCandidate> for lk_candidate::IceCandidate {
    type Error = String;
    fn try_from(c: IceCandidate) -> Result<Self, Self::Error> {
        lk_candidate::IceCandidate::create(
            &c.sdp_mid,
            c.sdp_mline_index,
            &c.candidate,
        ).map_err(|e| format!("invalid ICE candidate: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ice_candidate() {
        let lk = lk_candidate::IceCandidate::create("0", 0,
            "candidate:1 1 UDP 2130706431 127.0.0.1 8080 typ host")
            .expect("create ICE");
        let ours: IceCandidate = lk.into();
        assert_eq!(ours.sdp_mid, "0");
        assert!(ours.candidate.contains("typ host"));

        let back: lk_candidate::IceCandidate = ours.try_into().expect("back");
        assert_eq!(back.sdp_mid(), "0");
    }
}
```

- [ ] **Step 4: 更新 mod.rs 添加新模块**

修改 `livekit_rs/mod.rs`：

```rust
mod factory;
mod peer_connection;
mod session_description;
mod ice;

pub use factory::LiveKitRsFactory;
```

- [ ] **Step 5: 实现 peer_connection.rs 中的 SDP/ICE 方法**

更新 `LiveKitPeerConnection` 的 `impl PeerConnection`：

```rust
impl PeerConnection for LiveKitPeerConnection {
    // ... 之前的 state 方法保持不变 ...

    fn create_offer(&self) -> MediaResult<SessionDescription> {
        let offer = self.inner.create_offer()
            .map_err(|e| MediaError::new(format!("create_offer failed: {e}")))?;
        Ok(offer.into())
    }

    fn create_answer(&self) -> MediaResult<SessionDescription> {
        let answer = self.inner.create_answer()
            .map_err(|e| MediaError::new(format!("create_answer failed: {e}")))?;
        Ok(answer.into())
    }

    fn set_local_description(&self, sd: SessionDescription) -> MediaResult<()> {
        let lk_sd: lk_jsep::SessionDescription = sd.try_into()
            .map_err(|e| MediaError::new(e))?;
        self.inner.set_local_description(lk_sd)
            .map_err(|e| MediaError::new(format!("set_local_description: {e}")))
    }

    fn set_remote_description(&self, sd: SessionDescription) -> MediaResult<()> {
        let lk_sd: lk_jsep::SessionDescription = sd.try_into()
            .map_err(|e| MediaError::new(e))?;
        self.inner.set_remote_description(lk_sd)
            .map_err(|e| MediaError::new(format!("set_remote_description: {e}")))
    }

    fn add_ice_candidate(&self, c: IceCandidate) -> MediaResult<()> {
        let lk_c: lk_candidate::IceCandidate = c.try_into()
            .map_err(|e| MediaError::new(e))?;
        self.inner.add_ice_candidate(lk_c)
            .map_err(|e| MediaError::new(format!("add_ice_candidate: {e}")))
    }

    // ... 其余 todo!() 方法保持 ...
}
```

- [ ] **Step 6: 创建测试辅助函数**

创建 `tests/common/mod.rs`：

```rust
#![allow(dead_code)]

use std::time::{Duration, Instant};
use gkit_media::protocols::rtc::client::core::{
    PeerConnection, IceConnectionState, ConnectionState,
};

pub async fn wait_for_ice_connected(
    pc1: &dyn PeerConnection,
    pc2: &dyn PeerConnection,
    timeout: Duration,
) -> Result<(), String> {
    let start = Instant::now();
    loop {
        if start.elapsed() > timeout {
            return Err("timeout waiting for ICE connected".into());
        }
        let s1 = pc1.ice_connection_state();
        let s2 = pc2.ice_connection_state();

        if matches!(s1, IceConnectionState::Connected | IceConnectionState::Completed)
            && matches!(s2, IceConnectionState::Connected | IceConnectionState::Completed)
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

- [ ] **Step 7: 写 L1 P2P 集成测试**

创建 `tests/webrtc_lk_p2p.rs`：

```rust
#[cfg(feature = "backend-native-google")]
mod lk_p2p_tests {
    use std::time::Duration;
    use gkit_media::protocols::rtc::client::core::{
        PeerConnection, PeerConnectionFactory, ConnectionState,
        IceConnectionState,
    };
    use gkit_media::protocols::rtc::client::RtcEngine;
    use crate::common::wait_for_ice_connected;

    #[tokio::test]
    async fn p2p_offer_answer() {
        let factory = RtcEngine::create("google").expect("google backend not registered");

        let pc1 = factory.create_peer_connection().expect("pc1");
        let pc2 = factory.create_peer_connection().expect("pc2");

        // SDP 交换
        let offer = pc1.create_offer().expect("create_offer");
        pc1.set_local_description(offer.clone()).expect("pc1 set_local");
        pc2.set_remote_description(offer).expect("pc2 set_remote");
        let answer = pc2.create_answer().expect("create_answer");
        pc2.set_local_description(answer.clone()).expect("pc2 set_local");
        pc1.set_remote_description(answer).expect("pc1 set_remote");

        // 等待 ICE 建连
        wait_for_ice_connected(pc1.as_ref(), pc2.as_ref(), Duration::from_secs(15))
            .await
            .expect("ICE connection");

        assert!(matches!(
            pc1.connection_state(),
            ConnectionState::Connected | ConnectionState::Connecting
        ));

        pc1.close().unwrap();
        pc2.close().unwrap();
    }
}
```

- [ ] **Step 8: 跑 L1 P2P 测试**

```bash
cargo test --features backend-native-google webrtc_lk_p2p -- --nocapture 2>&1
```

- [ ] **Step 9: 提交**

```bash
git add crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/
git add crates/gkit-media/tests/common/
git add crates/gkit-media/tests/webrtc_lk_p2p.rs
git commit -m "feat: livekit_rs SDP/ICE adapter + P2P integration test"
```

---

### Task 10: VideoTrack + VideoFrame adapter (L1 推拉流测试)

**Files:**
- Create: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/video_track.rs`
- Create: `crates/gkit-media/src/protocols/rtc/client/native/livekit_rs/video_frame.rs`
- Create: `crates/gkit-media/tests/webrtc_lk_track.rs`

计划 10-14 将在下一阶段细化（模式与 Task 7-9 相同：先写 L0 单测 → FAIL → 实现 → PASS → L1 集成测试 → 提交）。

---

## 待细化的后续 Task

| Task | 内容 |
|------|------|
| 10 | VideoTrack + VideoFrame (L0 From/Into 单测 + L1 推拉流) |
| 11 | DataChannel adapter (L0 + L1 send/recv) |
| 12 | AudioTrack + AudioSource adapter (L1 PCM 推送) |
| 13 | FrameCryptor + Stats + RTP (L1 加密 + 统计) |
| 14 | DesktopCapturer (L1 桌面采集) |
| 15 | 清理 CMakeLists.txt + .cargo/config.toml + 全部测试通过 |
| 16 | egui P2P 可视化 example |
