# P2P Video Pipeline Design (VideoTrack + H.264)

**Date**: 2026-05-07
**Scope**: Add `VideoTrack` trait, `createVideoTrack`/`onTrack` W3C API, OpenH264 codec integration, P2P loopback example with real RTP frame transfer
**Backend**: webrtc-rs 0.17 + openh264 0.6
**Constraint**: API traits in `core.rs` use clean naming (no backend prefix); backend implementations may use `Wrtc*` prefix

---

## 1. Architecture

```
PC1 (sender):
  VideoFrameGenerator (VideoSource) → createVideoTrack(source) → openh264 Encoder → write_sample → RTP

PC2 (receiver):
  RTP → on_track → openh264 Decoder → I420Buffer → VideoSink.on_frame() → RGBA → egui
```

---

## 2. Core Trait Changes (`core.rs`)

### 2.1 Remove existing `VideoTrack` struct

Delete the current `VideoTrack` struct (with `id`, `kind`, `write_fn` fields).

### 2.2 New `VideoTrack` trait

```rust
/// Video track — W3C MediaStreamTrack-like abstraction.
/// Sender side: created via `create_video_track(source)`, internally bridges VideoSource → RTP.
/// Receiver side: obtained via `set_on_track` callback, `add_sink()` registers display consumers.
pub trait VideoTrack: Send {
    fn id(&self) -> &str;
    fn kind(&self) -> &str;
    /// Register a sink to receive decoded video frames (receiver side).
    fn add_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>);
}
```

### 2.3 Updated `PeerConnection` trait

Replace `add_track(&self, Arc<VideoTrack>)` and `set_on_track(&mut self, ...)` with:

```rust
/// Create a local video track backed by the given source (sender side).
/// The source's frames are encoded and sent via RTP.
fn create_video_track(&self, source: Box<dyn VideoSource<BoxVideoFrame>>)
    -> MediaResult<Box<dyn VideoTrack>> { Err(MediaError::new("not supported")) }

/// Register callback for incoming remote video tracks (receiver side).
fn set_on_track(&self, cb: Box<dyn Fn(Box<dyn VideoTrack>) + Send>) {}
```

**Existing signatures retained**: `set_on_ice_candidate(&self, ...)`, `set_on_ice_connection_state_change(&self, ...)`, `gather_complete()`.

---

## 3. Backend Implementation (`webrtc_rs_impl.rs`)

### 3.1 `WrtcVideoTrack` struct

```rust
#[cfg(feature = "backend-native-webrtc-rs")]
struct WrtcVideoTrack {
    id: String,
    tls: Option<Arc<TrackLocalStaticSample>>,  // sender side
    sinks: Mutex<Vec<Box<dyn VideoSink<BoxVideoFrame>>>>,
    encoder: Option<Mutex<openh264::encoder::Encoder>>>,   // sender side
    decoder: Option<Mutex<openh264::decoder::Decoder>>>,   // receiver side
}

#[cfg(feature = "backend-native-webrtc-rs")]
impl VideoTrack for WrtcVideoTrack {
    fn id(&self) -> &str { &self.id }
    fn kind(&self) -> &str { "video" }
    fn add_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>) {
        self.sinks.lock().unwrap().push(sink);
    }
}
```

### 3.2 Sender path (`create_video_track`)

```rust
fn create_video_track(&self, source: Box<dyn VideoSource<BoxVideoFrame>>) -> MediaResult<Box<dyn VideoTrack>> {
    let tls = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability { mime_type: MIME_TYPE_H264.to_string(), ..Default::default() },
        "video".into(), "sender".into(),
    ));
    let track = Arc::new(WrtcVideoTrack { id: "video0".into(), tls: Some(tls.clone()),
        sinks: Mutex::new(Vec::new()), encoder: Some(Mutex::new(openh264::encoder::Encoder::new()?)), decoder: None });

    // Internal: register a sink on the source to feed the encoder
    let tls_clone = tls.clone();
    source.add_or_update_sink(Box::new(EncoderSink { tls: tls_clone }),
        VideoSinkWants { is_active: true, ..Default::default() });
    // EncoderSink::on_frame(frame) → I420 → H.264 encode → write_sample

    rt().block_on(self.pc.add_track(tls))?;
    Ok(Box::new(track))
}
```

### 3.3 Receiver path (`set_on_track`)

```rust
fn set_on_track(&self, cb: Box<dyn Fn(Box<dyn VideoTrack>) + Send>) {
    let cb = Arc::new(Mutex::new(Some(cb)));
    self.pc.on_track(Box::new(move |track, _receiver, _transceiver| {
        let decoder = openh264::decoder::Decoder::new()?;
        let video_track = Arc::new(WrtcVideoTrack { id: track.id().into(),
            tls: None, sinks: Mutex::new(Vec::new()),
            encoder: None, decoder: Some(Mutex::new(decoder)) });
        if let Some(ref cb) = *cb.lock().unwrap() { cb(Box::new(video_track.clone())); }

        // spawn decoder + feed sinks thread
        tokio::spawn(async move {
            while let Ok((rtp, _)) = track.read_rtp().await {
                let dec = video_track.decoder.as_ref().unwrap().lock().unwrap();
                if let Some(i420) = dec.decode(&rtp.payload)? {
                    let frame = BoxVideoFrame::new(Box::new(i420));
                    for sink in video_track.sinks.lock().unwrap().iter() { sink.on_frame(&frame); }
                }
            }
        });
        Box::pin(async {})
    }));
}
```

---

## 4. Example Design (`gkit-media-webrtc-loopback`)

```rust
// PC1: sender
let mut pc1 = factory.create_peer_connection()?;
let generator = VideoFrameGenerator::new(640, 360, 15);
let track = pc1.create_video_track(Box::new(generator))?;
// ICE + SDP exchange ...

// PC2: receiver
pc2.set_on_track(Box::new(move |track| {
    let p = pipeline.clone();
    struct DisplaySink { p: Arc<Pipeline> }
    impl VideoSink<BoxVideoFrame> for DisplaySink { /* I420 → RGBA → store */ }
    track.add_sink(Box::new(DisplaySink { p }));
}));
```

**UI**: 左右分栏，各自显示发送帧 + 接收帧 + ICE/Connection 状态日志。接收帧来自真实 RTP → H.264 解码。

---

## 5. Dependencies

```toml
[dependencies]
openh264 = { version = "0.6", optional = true }

[features]
backend-native-webrtc-rs = ["backend-native", "dep:webrtc", "dep:tokio", "dep:bytes", "dep:openh264"]
```

`openh264` crate 已封装 Cisco OpenH264 编解码器，无需额外 C 编译。

---

## 6. Testing

| Test | Pattern | Description |
|------|---------|-------------|
| `track_create_local` | W3C addTrack | `create_video_track(source)` succeeds |
| `track_receiver_add_sink` | W3C ontrack | `set_on_track` callback delivers track, `add_sink` works |
| `track_encode_decode_roundtrip` | — | I420 → H.264 → I420 roundtrip |
| `p2p_video_transfer` | Echo Test | Full PC1→PC2 video frame transfer, verify received count > 0 |

---

## 7. Non-Goals

- Audio codec integration (only video/H.264)
- Hardware encoder (NVIDIA NVENC, VAAPI) — OpenH264 software only
- VP9/AV1 codecs
- Simulcast/SVC
- Screen capture
