# Google libwebrtc 后端从自维护 CXX FFI 迁移到 LiveKit rust-sdks

**Date**: 2026-05-25
**Status**: ✅ Completed (实施完毕，2026-05-25)
**Scope**: 删除自维护的 `build-sys/webrtc-sys/` (123 文件) 和 `google_lk/` (25 模块)，替换为依赖 LiveKit rust-sdks 的 `libwebrtc` + `yuv-sys` crate，新写 `livekit_rs` adapter 模块
**Constraint**: `core.rs` trait 层不动；RtcEngine 注册机制不动；所有代码在 `gkit-media` crate 内

> **Predecessor**: [WebRTC Backend Implementation Design](2026-05-06-webrtc-backend-implementation-design.md) — 定义了 google_lk 后端的原始架构
> **Predecessor**: [RtcEngine Factory Multi-Backend Design](2026-05-07-rtc-engine-factory-multi-backend-design.md) — 定义了多后端工厂注册模式

---

## 1. 动机

当前 Google libwebrtc 后端的维护成本过高：

| 项目 | 自维护方案 | LiveKit rust-sdks 方案 |
|------|----------|----------------------|
| 胶水文件数 | 123 (28 .rs + 98 C++/ObjC) | 0 |
| adapter 模块数 | 25 | ~10 |
| 维护的 C++ 代码 | ~23K 行 | 0 |
| unsafe 块 | 170+ | 0（在 adapter 层） |
| libwebrtc 构建 | CMake + vcpkg 源码编译 | 预编译二进制自动下载 |
| 上游更新 | 手动同步 | Cargo.toml 升级版本号 |

LiveKit 的 `libwebrtc` crate 已经解决了 GenericKit 正在自己解决的问题——通过 CXX FFI 桥接 libwebrtc C++ API。复用这个现有方案可以彻底删除自维护的 C++ 胶水层。

**核心原则**：`core.rs` trait 保持不变，接口与三方库解耦。新后端通过 `From`/`Into` 做类型转换。

---

## 2. 架构对比

### 2.1 现状 (google_lk)

```
你的 core.rs trait (PeerConnection, DataChannel, VideoTrack, ...)
  ↓ impl
google_lk/*.rs (25 模块, ~7K 行)
  ↓ 包装
google_lk/native/*.rs (CXX observer 桥接, SDP 解析, VideoFrame 类型层次)
  ↓ CXX FFI
build-sys/webrtc-sys/*.rs (28 CXX bridge 声明文件)
build-sys/webrtc-sys/*.cpp (98 C++ 实现文件)
  ↓ 链接
Google libwebrtc (CMake + vcpkg 源码编译或预编译二进制)
```

### 2.2 目标 (livekit_rs)

```
你的 core.rs trait (PeerConnection, DataChannel, VideoTrack, ...) ← 不动
  ↓ impl
livekit_rs/*.rs (~10 个 adapter 模块, ~3K 行)  ← 新写
  ↓ 委托 (Cargo.toml dep)
LiveKit libwebrtc 0.3.x (外部 crate, 你不需要维护)
  ↓ 内部
LiveKit webrtc-sys (CXX bridge, 你不需要维护)
  ↓ 下载
预编译 libwebrtc 二进制 (GitHub Releases, 自动下载 ~100MB)
```

### 2.3 模块对比

| 原 google_lk 模块 | 新 livekit_rs 模块 | 变更说明 |
|-------------------|-------------------|---------|
| `peer_connection_factory.rs` | `factory.rs` | 委托 libwebrtc 的 PCF |
| `peer_connection.rs` (619 行) | `peer_connection.rs` | 19 observer 回调映射 |
| `data_channel.rs` | `data_channel.rs` | 直接映射 |
| `video_track.rs` | `video_track.rs` | 直接映射 |
| `audio_track.rs` | `audio_track.rs` | 直接映射 |
| `video_frame.rs` (555+894 行) | `video_frame.rs` | From/Into 转换 7 种 buffer |
| `yuv_helper.rs` (868 行) | **删除** | 改用 LiveKit yuv-sys |
| `frame_cryptor.rs` (316 行) | `frame_cryptor.rs` | 委托 libwebrtc FrameCryptor |
| `packet_trailer.rs` (134 行) | `packet_trailer.rs` | 委托 libwebrtc PacketTrailer |
| `audio_source.rs` (205 行) | `audio_source.rs` | 委托 libwebrtc AudioSource |
| `session_description.rs` | `session_description.rs` | SDP 类型映射 |
| `ice_candidate.rs` + `jsep` | `ice.rs` | ICE/SDP 合并 |
| `rtp_*.rs` (6 模块) | `rtp.rs` | RTP 参数/收发器合并 |
| `stats.rs` (624 行) | `stats.rs` | getStats 委托 |
| `desktop_capturer.rs` | `desktop_capturer.rs` | 委托 libwebrtc DesktopCapturer |
| `build-sys/webrtc-sys/` (123 文件) | **整个目录删除** | — |
| `build-sys/yuv-sys/` | **删除** | 改用 LiveKit yuv-sys |

---

## 3. 类型转换设计

核心模式：Newtype + `From`/`Into` 双向转换。

### 3.1 Adapter 结构

```rust
// livekit_rs/peer_connection.rs
use libwebrtc::native::peer_connection as lk;
use crate::protocols::rtc::client::core::PeerConnection;

pub struct LiveKitPeerConnection {
    inner: lk::PeerConnection,
    observer: Option<lk::PeerConnectionObserver>,
}

// 实现 libwebrtc 的 Observer trait，转发到 gkit 的事件通道
impl lk::PeerConnectionObserver for GkitPcObserver {
    fn on_ice_candidate(&self, candidate: lk::IceCandidate) {
        self.tx.send(RtcEvent::IceCandidate(candidate.into())).ok();
    }
    fn on_connection_change(&self, state: lk::PeerConnectionState) {
        self.tx.send(RtcEvent::ConnectionState(state.into())).ok();
    }
    // ... 19 个回调
}

// 实现你的 trait
impl PeerConnection for LiveKitPeerConnection {
    fn create_offer(&self) -> MediaResult<SessionDescription> {
        Ok(self.inner.create_offer()?.into())
    }
    fn set_remote_description(&self, sd: SessionDescription) -> MediaResult<()> {
        self.inner.set_remote_description(sd.into())
    }
}
```

### 3.2 类型映射表

| 你的类型 | libwebrtc 类型 | 方向 | 复杂度 |
|---------|---------------|------|--------|
| `SessionDescription` | `lk::SessionDescription` | ↔ | 低 |
| `IceCandidate` | `lk::IceCandidate` | ↔ | 低 |
| `VideoFrame` | `lk::VideoFrame` | ↔ | **高** |
| `VideoBuffer` (7 子类型) | `lk::VideoFrameBuffer` | ↔ | **高** |
| `RtcConfiguration` | `lk::RtcConfiguration` | → | 低 |
| `DataChannelInit` | `lk::DataChannelInit` | → | 低 |
| `Rotation` | `lk::VideoRotation` | ↔ | 低 |
| `ConnectionState` | `lk::PeerConnectionState` | ↔ | 低 |
| `IceConnectionState` | `lk::IceConnectionState` | ↔ | 低 |
| `RtcStats` | JSON `serde_json::Value` | → | 中 |

### 3.3 VideoFrame 转换（关键路径）

```rust
// 你的 VideoFrame → LiveKit VideoFrame (打包，如发送前)
impl From<gkit_media::video::VideoFrame> for lk::VideoFrame {
    fn from(f: gkit_media::video::VideoFrame) -> Self {
        match f.buffer() {
            VideoBuffer::I420(buf) => {
                let lk_buf = lk::I420Buffer::new(buf.width(), buf.height());
                // 拷贝数据 plane-by-plane
                lk_buf.data_y_mut().copy_from_slice(buf.data_y());
                lk_buf.data_u_mut().copy_from_slice(buf.data_u());
                lk_buf.data_v_mut().copy_from_slice(buf.data_v());
                lk::VideoFrame::builder()
                    .set_video_frame_buffer(lk_buf)
                    .set_rotation(f.rotation().into())
                    .set_timestamp_us(f.timestamp_us())
                    .build()
            }
            VideoBuffer::NV12(buf) => {
                let lk_buf = lk::NV12Buffer::new(buf.width(), buf.height());
                lk_buf.data_y_mut().copy_from_slice(buf.data_y());
                lk_buf.data_uv_mut().copy_from_slice(buf.data_uv());
                lk::VideoFrame::builder()
                    .set_video_frame_buffer(lk_buf)
                    .set_rotation(f.rotation().into())
                    .set_timestamp_us(f.timestamp_us())
                    .build()
            }
            // ... I422, I444, I010, Native
        }
    }
}

// LiveKit VideoFrame → 你的 VideoFrame (解包，如接收后)
impl From<lk::VideoFrame> for gkit_media::video::VideoFrame {
    fn from(f: lk::VideoFrame) -> Self {
        let buf = f.video_frame_buffer();
        match buf.buffer_type() {
            lk::BufferType::I420 => {
                let i420 = buf.to_i420();
                gkit_media::video::VideoFrame::from_i420(
                    i420.width(), i420.height(),
                    i420.data_y(), i420.data_u(), i420.data_v(),
                    i420.stride_y(), i420.stride_u(), i420.stride_v(),
                )
            }
            lk::BufferType::NV12 => {
                let nv12 = buf.to_nv12();
                // ...
            }
            // ... I422, I444, I010, Native
        }
    }
}
```

---

## 4. 依赖变更

### 4.1 Cargo.toml

```toml
# ===== 删除 =====
# 原 backend-native-google 的 14 个依赖全部删除
# cxx, parking_lot, thiserror, serde, serde_json, log, lazy_static, 
# futures, rtrb, enum_dispatch, scoped-tls (如未被其他 feature 共用)

# ===== 新增 =====
[dependencies]
libwebrtc = { git = "https://github.com/livekit/rust-sdks", tag = "libwebrtc-v0.3.34" }
yuv-sys   = { git = "https://github.com/livekit/rust-sdks", tag = "yuv-sys-v0.3.14" }

[features]
backend-native-google = ["libwebrtc"]   # 从 14 个依赖缩到 1 个
```

### 4.2 CMakeLists.txt

- 删除 `build-sys/webrtc-sys/` 下所有 C++ 文件编译配置
- 删除 vcpkg 中 libwebrtc 相关包安装
- 删除 `corrosion_import_crate` 中 `backend-native-google` 的额外 target properties
- 保留 `gkit_media` corrosion target

### 4.3 .cargo/config.toml

合并 LiveKit 需要的 linker flags：
```toml
[target.aarch64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-args=-ObjC"]
```

---

## 5. Engine 注册（不变）

```rust
// protocol/rtc/client/native/mod.rs
#[cfg(feature = "backend-native-google")]
mod livekit_rs;

#[cfg(feature = "backend-native-google")]
gkit_register_rtc_backend!(livekit_rs::LiveKitRsFactory, backend_native_google);
```

注册机制、`create_default()` 优先级逻辑均不变。`RtcEngine` 既不感知是 `google_lk` 还是 `livekit_rs`。

---

## 6. 测试策略

### 6.1 三层测试架构

| 层级 | 类型 | 位置 | 依赖 | 验证目标 |
|------|------|------|------|---------|
| L0 | 单测 | `livekit_rs/*.rs` `#[cfg(test)]` | 无 | From/Into 类型转换正确性 |
| L1 | 集成测试 | `tests/webrtc_*.rs` | 真实 libwebrtc 后端 | PC 生命周期、P2P 建连、Track 推拉 |
| L2 | 可视化验证 | `examples/gkit-media-webrtc-p2p-loopback/` | 真实 libwebrtc 后端 | 端到端推拉流 + 状态监控 |

**不使用 mock，所有测试走真实 `libwebrtc` 后端**。

### 6.2 L0: 类型转换单测

```rust
// livekit_rs/video_frame.rs
#[cfg(test)]
mod tests {
    #[test]
    fn roundtrip_i420_frame() {
        let ours = make_test_i420_frame(640, 480);
        let lk: lk::VideoFrame = ours.clone().into();
        let back: gkit_media::VideoFrame = lk.into();
        assert_frame_eq(&ours, &back);
    }

    #[test]
    fn roundtrip_session_description() { /* SDP 文本往返不变 */ }
    #[test]
    fn roundtrip_ice_candidate() { /* candidate 字符串往返不变 */ }
    #[test]
    fn rotation_mapping() { /* Rotation 双向映射覆盖全枚举 */ }
}
```

### 6.3 L1: 基于 trait 的集成测试

测试不依赖具体后端类型，依赖 `core.rs` trait。用 feature flag 选择后端：

```rust
// tests/webrtc_p2p.rs
#[cfg(feature = "backend-native-google")]
#[tokio::test]
async fn p2p_connection_established() {
    let engine = RtcEngine::create("google")?;
    let (pc1, pc2) = create_p2p_pair(&engine).await?;

    // SDP 交换
    let offer = pc1.create_offer()?;
    pc1.set_local_description(offer.clone())?;
    pc2.set_remote_description(offer)?;
    let answer = pc2.create_answer()?;
    pc2.set_local_description(answer.clone())?;
    pc1.set_remote_description(answer)?;

    // 等待 ICE 建连
    wait_for_ice_connected(&pc1, &pc2, Duration::from_secs(15)).await?;
    assert_eq!(pc1.connection_state(), ConnectionState::Connected);
}

#[cfg(feature = "backend-native-google")]
#[tokio::test]
async fn video_track_push_pull() {
    let (pc1, pc2) = create_p2p_pair(&engine).await?;
    let source = VideoSource::new(VideoResolution { width: 640, height: 480 });
    let track = source.create_track("video")?;
    pc1.add_track(track)?;

    let sink = Arc::new(TestVideoSink::new());
    pc2.on_track(move |t| { t.add_sink(sink.clone()); });

    // 推一帧
    let frame = make_test_i420_frame(640, 480);
    source.push_frame(frame.clone())?;

    // 等待接收
    let received = sink.wait_for_frame(Duration::from_secs(5)).await?;
    assert_frame_eq(&frame, &received);
}
```

**测试辅助工具** (`tests/common/`):

| 工具 | 作用 |
|------|------|
| `create_p2p_pair(engine)` | 创建配对 PC1/PC2，配置回环 STUN |
| `wait_for_ice_connected(pc1, pc2, timeout)` | 轮询 ICE 状态直到 Connected |
| `TestVideoSink` | 收帧并支持 `wait_for_frame()` |
| `TestAudioSink` | 收音频数据 |
| `make_test_i420_frame(w, h)` | 生成标准 I420 测试帧 |
| `assert_frame_eq(a, b)` | 像素级帧比较 |
| `RtcEventLog` | 收集 SDP/ICE/状态变化事件 |

**功能覆盖矩阵**:

| 测试场景 | 验证点 |
|---------|--------|
| `pc_lifecycle` | 创建 → Configure → Close |
| `p2p_offer_answer` | SDP 生成、交换、设置 |
| `p2p_ice_connection` | ICE candidate 交换、状态迁移 (15s 超时) |
| `video_track_push_pull` | 推一帧 → 收一帧 → 数据一致性 |
| `video_multi_frame` | 连续推 N 帧、顺序校验 |
| `audio_track_loopback` | PCM 推送 → 接收校验 |
| `data_channel_text` | send/recv 文本消息 |
| `data_channel_binary` | send/recv 二进制 |
| `error_handling` | 无效 SDP、重复 close、空 track |
| `renegotiation` | 建连后 addTrack/removeTrack |
| `stats_collection` | getStats 返回有效 JSON |
| `frame_cryptor` | 加密/解密往返 |

### 6.4 L2: egui 可视化 P2P 推拉流

```
examples/gkit-media-webrtc-p2p-loopback/
├── main.rs            # egui 主窗口
├── p2p_session.rs     # P2P 会话管理
├── video_source.rs    # 帧生成器（渐变/色条/雪花）
├── state_panel.rs     # 状态面板
└── Cargo.toml
```

**egui 布局**:

```
┌──────────────────────────────────────────────────────┐
│  WebRTC P2P Loopback Tester                          │
├────────────────────┬─────────────────────────────────┤
│  📹 Sender (PC1)   │  📹 Receiver (PC2)              │
│  [推流画面 640x480] │  [拉流画面 640x480]              │
│  模式: ▸渐变        │  帧率: 29.8 fps                 │
│  已推: 1,234 帧    │  已收: 1,233 帧                  │
├────────────────────┴─────────────────────────────────┤
│  📡 Signaling Log                                    │
│  [09:30:01] PC1: createOffer                         │
│  [09:30:01] PC2: setRemoteDescription (offer)        │
│  [09:30:02] PC1 ICE: gathering → complete             │
│  [09:30:03] PC2 ICE: checking → connected             │
├──────────────────────────────────────────────────────┤
│  📊 States  PC1: 🟢   PC2: 🟢  │  RTT: 2.1ms         │
│  [▶ Start]  [⏸ Pause]  [🔄 Restart]  [💾 SDP]       │
└──────────────────────────────────────────────────────┘
```

**信令事件日志类型**:

```rust
enum SignalingEvent {
    CreateOffer { sdp: String },
    CreateAnswer { sdp: String },
    SetLocalDescription { ty: SdpType },
    SetRemoteDescription { ty: SdpType },
    IceGatheringState(GatheringState),
    IceConnectionState(IceConnectionState),
    IceCandidate { sdp_mid: String, candidate: String },
    ConnectionState(ConnectionState),
    TrackAdded { id: String, kind: MediaKind },
    FrameSent { track_id: String, timestamp: u64 },
    FrameReceived { track_id: String, timestamp: u64 },
}
```

---

## 7. TDD 实施顺序

| 阶段 | 做什么 | 验证标准 | 依赖 |
|------|--------|---------|------|
| **P0** | 删除旧代码 + 加 `libwebrtc` dep | `cargo check -p gkit-media --features backend-native-google` | 无 |
| **P1** | 写 L0 类型转换单测 | `cargo test --lib --features backend-native-google` | P0 |
| **P2** | 实现 `factory.rs` + `peer_connection.rs` adapter | P1 + L1 `pc_lifecycle` 通过 | P1 |
| **P3** | 实现 SDP/ICE adapter + P2P 集成测试 | `p2p_offer_answer` + `p2p_ice` 通过 | P2 |
| **P4** | 实现 VideoTrack adapter + 推拉流测试 | `video_track_push_pull` 通过 | P3 |
| **P5** | 实现 DataChannel + AudioTrack adapter | 对应集成测试通过 | P3 |
| **P6** | 实现 FrameCryptor + Stats + DesktopCapturer | 对应测试通过 | P4 |
| **P7** | egui P2P 可视化验证 | 手动 `cargo run --example` 验证 | P5 |

---

## 8. 风险与缓解

| 风险 | 可能性 | 缓解措施 |
|------|--------|---------|
| LiveKit `libwebrtc` API 与 `core.rs` trait 不完全匹配 | 中 | 写 adapter 层时发现，逐个解决 |
| 预编译 libwebrtc 平台覆盖不足（如 Jetson） | 低 | 后期 fork webrtc-sys-build 加自定义构建 |
| libwebrtc 版本更新不兼容 | 低 | 用 tag 锁定版本，升级前跑 CI |
| PacketTrailer 未在 `libwebrtc` 暴露 | 中 | 待确认，必要时提 PR 到 LiveKit |
| VideoFrame buffer 拷贝开销 | 低 | 可接受；如成瓶颈则优化零拷贝路径 |

---

## 9. 受影响的历史文档

以下 spec 需添加状态更新说明本次迁移：

- `2026-05-06-webrtc-backend-implementation-design.md` — `google_lk` → `livekit_rs`
- `2026-05-07-rtc-engine-factory-multi-backend-design.md` — `google_lk/` → `livekit_rs/`
