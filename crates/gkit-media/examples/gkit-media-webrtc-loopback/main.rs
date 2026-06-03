// gkit-media P2P Video Loopback (egui) — plugin-based backend discovery
// Usage:
//   cargo build -p gkit-plugin-webrtc-libwebrtc
//   cargo run -p gkit-media --example gkit-media-webrtc-loopback [-- --auto-start]
//
// Options:
//   --auto-start    Start P2P immediately on launch (default: off)
//
// Backends are discovered dynamically via RtcEngine::load_plugins().

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;
use std::time::Duration;
use eframe::egui;
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::protocols::rtc::peer::{
    RtcConfiguration, 
    VideoTrack,
    IceConnectionState, 
    IceCandidate, 
    IceServer,
};
use gkit_media::protocols::rtc::peer::RtcEngine;
use gkit_media::video::buffer::VideoFormatType;
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};
use gkit_media::video::frame::BoxVideoFrame;
use tokio::runtime::Runtime;

/// Wraps an `Arc<VideoFrameGenerator>` so it can be passed to `create_video_track`
/// which requires `Box<dyn VideoSource<BoxVideoFrame>>`.
struct ArcVideoSource {
    inner: Arc<VideoFrameGenerator>,
}
impl VideoSource<BoxVideoFrame> for ArcVideoSource {
    fn add_or_update_sink(&self, sink: Box<dyn VideoSink<BoxVideoFrame>>, wants: VideoSinkWants) {
        self.inner.add_or_update_sink(sink, wants);
    }
    fn remove_sink(&self, sink: &dyn VideoSink<BoxVideoFrame>) {
        self.inner.remove_sink(sink);
    }
}

const W: u32 = 640;
const H: u32 = 360;
const FPS: u32 = 15;

#[derive(Debug)]
enum P2pState {
    Idle,
    Connecting,
    Connected,
    Error,
}

struct Pipeline {
    sender_frame: Mutex<Option<(Vec<u8>, u32, u32)>>,
    receiver_frame: Mutex<Option<(Vec<u8>, u32, u32)>>,
    sender_count: Mutex<u64>,
    receiver_count: Mutex<u64>,
    pc1_log: Mutex<Vec<String>>,
    pc2_log: Mutex<Vec<String>>,
    status: Mutex<String>,
    p2p_state: Mutex<P2pState>,
    selected_backend: Mutex<String>,
    available_backends: Mutex<Vec<String>>,
    ice_config: RtcConfiguration,
    tokio_handle: tokio::runtime::Handle,
    stop_requested: AtomicBool,
    receiver_stats: Mutex<String>,
    generator: Arc<VideoFrameGenerator>,
}

fn default_ice_config() -> RtcConfiguration {
    RtcConfiguration {
        ice_servers: vec![IceServer {
            urls: vec!["stun:stun.l.google.com:19302".into()],
            username: None,
            credential: None,
        }],
        ..Default::default()
    }
}

fn main() -> Result<(), eframe::Error> {
    {
        let mut f = std::fs::File::create("/tmp/gkit_loopback.log").unwrap();
        writeln!(f, "loopback starting...").unwrap();
    }
    let tokio_rt = Runtime::new().expect("tokio runtime");
    let _rt_thread = {
        let h = tokio_rt.handle().clone();
        std::thread::spawn(move || h.block_on(std::future::pending::<()>()));
    };

    RtcEngine::load_plugins();
    let backends = RtcEngine::registered_types();
    {
        let mut f = std::fs::OpenOptions::new().append(true).open("/tmp/gkit_loopback.log").unwrap();
        writeln!(f, "plugins loaded, backends: {:?}", backends).unwrap();
    }
    let _ = std::fs::write("/tmp/gkit_startup.log", format!("loopback started\nbackends: {:?}\n", backends));
    let tokio_handle = tokio_rt.handle().clone();

    let default_backend = if backends.contains(&"libwebrtc".to_string()) {
        "libwebrtc".to_string()
    } else if backends.contains(&"webrtc-rs".to_string()) {
        "webrtc-rs".to_string()
    } else {
        backends.first().map(|s| s.clone()).unwrap_or_else(|| "none".into())
    };
    let auto_start = std::env::args().any(|a| a == "--auto-start")
        && default_backend != "none"
        && !default_backend.is_empty();

    // Shared frame generator — drives both sender display and video track
    let mut generator = VideoFrameGenerator::new(W, H, FPS);
    generator.start(); // must start before sharing (start() takes &mut self)
    let generator = Arc::new(generator);

    let pipeline = Arc::new(Pipeline {
        sender_frame: Mutex::new(None),
        receiver_frame: Mutex::new(None),
        sender_count: Mutex::new(0),
        receiver_count: Mutex::new(0),
        pc1_log: Mutex::new(Vec::new()),
        pc2_log: Mutex::new(Vec::new()),
        status: Mutex::new(format!("Select backend and press Start")),
        p2p_state: Mutex::new(P2pState::Idle),
        selected_backend: Mutex::new(default_backend.clone()),
        available_backends: Mutex::new(backends),
        ice_config: default_ice_config(),
        tokio_handle,
        stop_requested: AtomicBool::new(false),
        receiver_stats: Mutex::new(String::new()),
        generator: generator.clone(),
    });

    // Sender display sink — shows PC1's generated frames locally
    {
        struct LoopSink {
            p: Arc<Pipeline>,
        }
        impl VideoSink<BoxVideoFrame> for LoopSink {
            fn on_frame(&self, frame: &BoxVideoFrame) {
                if let Ok(i420) = frame.buffer.to_i420() {
                    let w = i420.width;
                    let h = i420.height;
                    let mut rgba = vec![0u8; (w * h * 4) as usize];
                    i420_to_argb(&i420, &mut rgba, w * 4, VideoFormatType::RGBA);
                    *self.p.sender_frame.lock().unwrap() = Some((rgba, w, h));
                    *self.p.sender_count.lock().unwrap() += 1;
                }
            }
        }
        generator.add_or_update_sink(
            Box::new(LoopSink { p: pipeline.clone() }),
            VideoSinkWants {
                is_active: true,
                ..Default::default()
            },
        );
    }

    if auto_start {
        let p = pipeline.clone();
        let handle = pipeline.tokio_handle.clone();
        let backend = default_backend.to_string();
        *p.p2p_state.lock().unwrap() = P2pState::Connecting;
        std::thread::spawn(move || run_p2p(p, backend, handle));
    }

    eframe::run_native(
        "gkit-media P2P Video Loopback",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 720.0]),
            ..Default::default()
        },
        Box::new(move |_cc| {
            struct App {
                p: Arc<Pipeline>,
                gt: Option<egui::TextureHandle>,
                rt: Option<egui::TextureHandle>,
            }
            impl eframe::App for App {
                fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
                    // ---- Top bar: status + controls ----
                    egui::TopBottomPanel::top("controls").show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Backend:");
                            let backends = self.p.available_backends.lock().unwrap().clone();
                            let mut selected = self.p.selected_backend.lock().unwrap().clone();

                            egui::ComboBox::from_id_salt("backend")
                                .width(120.0)
                                .selected_text(&selected)
                                .show_ui(ui, |ui| {
                                    for b in &backends {
                                        ui.selectable_value(&mut selected, b.clone(), b);
                                    }
                                });
                            *self.p.selected_backend.lock().unwrap() = selected.clone();

                            let is_idle = matches!(*self.p.p2p_state.lock().unwrap(), P2pState::Idle);
                            let can_start = is_idle && !selected.is_empty()
                                && !selected.eq("none");

                            ui.add_enabled_ui(can_start, |ui| {
                                if ui.button("▶ Start P2P").clicked() {
                                    let backend = selected.clone();
                                    let p = self.p.clone();
                                    *p.status.lock().unwrap() = format!("Starting: {} ...", backend);
                                    *p.p2p_state.lock().unwrap() = P2pState::Connecting;
                                    p.stop_requested.store(false, Ordering::Relaxed);
                                    p.pc1_log.lock().unwrap().clear();
                                    p.pc2_log.lock().unwrap().clear();
                                    let handle = p.tokio_handle.clone();
                                    std::thread::spawn(move || run_p2p(p, backend, handle));
                                }
                            });

                            let is_running = matches!(*self.p.p2p_state.lock().unwrap(), P2pState::Connecting | P2pState::Connected);
                            if is_running {
                                if ui.button("⏹ Stop P2P").clicked() {
                                    self.p.stop_requested.store(true, Ordering::Relaxed);
                                    *self.p.status.lock().unwrap() = "Stopping...".into();
                                    self.p.receiver_frame.lock().unwrap().take();
                                    self.p.pc2_log.lock().unwrap().clear();
                                    *self.p.receiver_count.lock().unwrap() = 0;
                                    self.rt.take();
                                }
                            }

                            ui.separator();
                            ui.label(format!("Status: {}", self.p.status.lock().unwrap()));
                        });
                    });

                    // ---- Center: video panels ----
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let sc = *self.p.sender_count.lock().unwrap();
                        let rc = *self.p.receiver_count.lock().unwrap();
                        ui.columns(2, |cols| {
                            // Sender (PC1)
                            cols[0].vertical_centered(|ui| {
                                ui.heading(format!("📹 Sender (PC1) — {} frames", sc));
                                if let Some(ref data) = *self.p.sender_frame.lock().unwrap() {
                                    let (rgba, fw, fh) = data;
                                    let fw = *fw as usize;
                                    let fh = *fh as usize;
                                    self.gt = Some(ctx.load_texture(
                                        "s",
                                        egui::ColorImage::from_rgba_unmultiplied(
                                            [fw, fh],
                                            rgba,
                                        ),
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                                if let Some(ref t) = self.gt {
                                    ui.image(egui::load::SizedTexture::new(
                                        t.id(),
                                        [ui.available_width().min(W as f32), (H as f32)],
                                    ));
                                }
                                ui.separator();
                                ui.label("📡 Sender Log");
                                egui::ScrollArea::vertical()
                                    .id_salt("log1")
                                    .show(ui, |ui| {
                                        for line in
                                            self.p.pc1_log.lock().unwrap().iter().rev().take(12)
                                        {
                                            ui.label(line);
                                        }
                                    });
                            });
                            // Receiver (PC2)
                            cols[1].vertical_centered(|ui| {
                                ui.heading(format!("📹 Receiver (PC2) — {} frames", rc));
                                if let Some(ref data) = *self.p.receiver_frame.lock().unwrap() {
                                    let (rgba, fw, fh) = data;
                                    let fw = *fw as usize;
                                    let fh = *fh as usize;
                                    self.rt = Some(ctx.load_texture(
                                        "r",
                                        egui::ColorImage::from_rgba_unmultiplied(
                                            [fw, fh],
                                            rgba,
                                        ),
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                                if let Some(ref t) = self.rt {
                                    ui.image(egui::load::SizedTexture::new(
                                        t.id(),
                                        [ui.available_width().min(W as f32), (H as f32)],
                                    ));
                                }
                                ui.separator();
                                ui.label(self.p.receiver_stats.lock().unwrap().clone());
                                ui.separator();
                                ui.label("📡 Receiver Log");
                                egui::ScrollArea::vertical()
                                    .id_salt("log2")
                                    .show(ui, |ui| {
                                        for line in
                                            self.p.pc2_log.lock().unwrap().iter().rev().take(12)
                                        {
                                            ui.label(line);
                                        }
                                    });
                            });
                        });
                    });

                    ctx.request_repaint();
                }
            }
            Ok(Box::new(App {
                p: pipeline,
                gt: None,
                rt: None,
            }))
        }),
    )
}

fn run_p2p(p: Arc<Pipeline>, backend: String, handle: tokio::runtime::Handle) {
    handle.block_on(async move {
        run_p2p_async(p, backend).await
    });
}

async fn run_p2p_async(p: Arc<Pipeline>, backend: String) {
    use std::io::Write;
    let log_file = std::fs::File::create("/tmp/gkit_loopback.log")
        .map(|f| std::sync::Mutex::new(f))
        .ok();
    let log = |peer: &str, msg: &str| {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() % 100000)
            .unwrap_or(0);
        let line = format!("[{:05}] {}: {}", ts, peer, msg);
        if let Some(ref f) = log_file {
            let _ = writeln!(f.lock().unwrap(), "{}", line);
        }
        match peer {
            "PC1" | "SYS" => p.pc1_log.lock().unwrap().push(line.clone()),
            "PC2" => p.pc2_log.lock().unwrap().push(line.clone()),
            _ => {}
        }
        if peer != "PC1" && peer != "PC2" {
            p.pc1_log.lock().unwrap().push(line.clone());
            p.pc2_log.lock().unwrap().push(line);
        }
    };

    let factory = match RtcEngine::create(&backend) {
        Ok(f) => {
            log("SYS", &format!("Backend '{}' loaded", backend));
            f
        }
        Err(e) => {
            *p.status.lock().unwrap() = format!("Error: {}", e);
            *p.p2p_state.lock().unwrap() = P2pState::Error;
            return;
        }
    };

    let config = p.ice_config.clone();
    let mut pc1 = match factory.create_peer_connection_with_config(&config) {
        Ok(pc) => {
            log("PC1", "PeerConnection created");
            pc
        }
        Err(e) => {
            *p.status.lock().unwrap() = format!("PC1 create error: {}", e);
            *p.p2p_state.lock().unwrap() = P2pState::Error;
            return;
        }
    };
    let mut pc2 = match factory.create_peer_connection_with_config(&config) {
        Ok(pc) => {
            log("PC2", "PeerConnection created");
            pc
        }
        Err(e) => {
            *p.status.lock().unwrap() = format!("PC2 create error: {}", e);
            *p.p2p_state.lock().unwrap() = P2pState::Error;
            pc1.close().ok();
            return;
        }
    };

    let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel::<IceCandidate>();
    let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel::<IceCandidate>();
    pc1.set_on_ice_candidate(Box::new(move |c| {
        let _ = tx2.send(c);
    }));
    pc2.set_on_ice_candidate(Box::new(move |c| {
        let _ = tx1.send(c);
    }));

    {
        let p1 = p.clone();
        pc1.set_on_ice_connection_state_change(Box::new(move |s| {
            p1.pc1_log.lock().unwrap().push(format!("ICE state: {:?}", s));
            eprintln!("[ICE] PC1 state: {:?}", s);
            if s == IceConnectionState::Connected {
                *p1.status.lock().unwrap() = "P2P Connected!".into();
                *p1.p2p_state.lock().unwrap() = P2pState::Connected;
            } else if s == IceConnectionState::Disconnected || s == IceConnectionState::Failed {
                eprintln!("[ICE] PC1 DISCONNECTED/FAILED: {:?}", s);
            }
        }));
    }
    {
        let p2 = p.clone();
        pc2.set_on_ice_connection_state_change(Box::new(move |s| {
            p2.pc2_log.lock().unwrap().push(format!("ICE state: {:?}", s));
            eprintln!("[ICE] PC2 state: {:?}", s);
            if s == IceConnectionState::Connected {
                *p2.status.lock().unwrap() = "P2P Connected!".into();
                *p2.p2p_state.lock().unwrap() = P2pState::Connected;
            } else if s == IceConnectionState::Disconnected || s == IceConnectionState::Failed {
                eprintln!("[ICE] PC2 DISCONNECTED/FAILED: {:?}", s);
            }
        }));
    }

    let source = ArcVideoSource { inner: p.generator.clone() };
    match pc1.create_video_track(Box::new(source)) {
        Ok(track) => {
            log("PC1", &format!("VideoTrack added: {}", track.id()));
        }
        Err(e) => {
            log("PC1", &format!("VideoTrack error: {}", e));
        }
    }

    let (track_tx, mut track_rx) = tokio::sync::mpsc::unbounded_channel::<Box<dyn VideoTrack>>();
    let rp = p.clone();
    pc2.set_on_track(Box::new(move |track: Box<dyn VideoTrack>| {
        rp.pc2_log.lock().unwrap().push(format!("[RX] Remote track received: id={}", track.id()));
        let _ = track_tx.send(track);
    }));

    *p.status.lock().unwrap() = "SDP negotiation...".into();
    if let Err(e) = (|| -> Result<(), String> {
        let offer = pc1.create_offer().map_err(|e| format!("offer: {e}"))?;
        log("SDP", "=== PC1 Offer ===");
        for line in offer.sdp.lines() {
            log("SDP", line);
        }
        log("SDP", "=== End Offer ===");
        pc1.set_local_description(&offer).map_err(|e| format!("setLocal1: {e}"))?;
        pc2.set_remote_description(&offer).map_err(|e| format!("setRemote2: {e}"))?;
        let answer = pc2.create_answer().map_err(|e| format!("answer: {e}"))?;
        log("SDP", "=== PC2 Answer ===");
        for line in answer.sdp.lines() {
            log("SDP", line);
        }
        log("SDP", "=== End Answer ===");
        pc2.set_local_description(&answer).map_err(|e| format!("setLocal2: {e}"))?;
        pc1.set_remote_description(&answer).map_err(|e| format!("setRemote1: {e}"))?;
        pc1.gather_complete().map_err(|e| format!("gather: {e}"))?;
        pc2.gather_complete().map_err(|e| format!("gather: {e}"))?;
        Ok(())
    })() {
        log("SDP", &format!("Error: {}", e));
        *p.p2p_state.lock().unwrap() = P2pState::Error;
        pc1.close().ok();
        pc2.close().ok();
        return;
    }

    *p.status.lock().unwrap() = format!("P2P active — {}x{} @ {}fps", W, H, FPS);
    struct P2PSink { p: Arc<Pipeline> }
    impl VideoSink<BoxVideoFrame> for P2PSink {
        fn on_frame(&self, frame: &BoxVideoFrame) {
            if let Ok(i420) = frame.buffer.to_i420() {
                let w = i420.width;
                let h = i420.height;
                let mut rgba = vec![0u8; (w * h * 4) as usize];
                i420_to_argb(&i420, &mut rgba, w * 4, VideoFormatType::RGBA);
                let count = { let mut c = self.p.receiver_count.lock().unwrap(); *c += 1; *c };
                *self.p.receiver_frame.lock().unwrap() = Some((rgba, w, h));
                if count <= 3 || count % 30 == 0 {
                    eprintln!("[RX] frame #{}: {}x{}", count, w, h);
                }
            }
        }
    }
    let start = std::time::Instant::now();
    loop {
        while let Ok(track) = track_rx.try_recv() {
            log("PC2", &format!("on_track: id={} kind={}", track.id(), track.kind()));
            let _ = std::fs::write("/tmp/gkit_track_received.log", "1");
            track.add_sink(Box::new(P2PSink { p: p.clone() }));
        }
        while let Ok(c) = rx2.try_recv() {
            pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok();
            log("ICE", &format!("PC1→PC2: mid={:?} mline={:?} candidate={}",
                c.sdp_mid, c.sdp_mline_index, &c.candidate[..c.candidate.len().min(80)]));
        }
        while let Ok(c) = rx1.try_recv() {
            pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok();
            log("ICE", &format!("PC2→PC1: mid={:?} mline={:?} candidate={}",
                c.sdp_mid, c.sdp_mline_index, &c.candidate[..c.candidate.len().min(80)]));
        }
        if start.elapsed() > Duration::from_secs(60)
            && matches!(*p.p2p_state.lock().unwrap(), P2pState::Connecting)
        {
            *p.status.lock().unwrap() = "ICE timeout (60s)".into();
            *p.p2p_state.lock().unwrap() = P2pState::Error;
            pc1.close().ok();
            pc2.close().ok();
            break;
        }
        if matches!(*p.p2p_state.lock().unwrap(), P2pState::Error) {
            pc1.close().ok();
            pc2.close().ok();
            break;
        }
        if p.stop_requested.load(Ordering::Relaxed) {
            log("SYS", "Stop requested — closing connections");
            *p.status.lock().unwrap() = "Stopped".into();
            *p.p2p_state.lock().unwrap() = P2pState::Idle;
            pc1.close().ok();
            pc2.close().ok();
            break;
        }

        if start.elapsed().as_secs() % 2 == 0 {
            let elapsed = start.elapsed().as_secs_f64().max(0.1);
            let rc = *p.receiver_count.lock().unwrap();
            let fps = rc as f64 / elapsed;
            let kbps = fps * 640.0 * 360.0 * 1.5 / 1000.0;
            let mut stats = format!("fps:{:.0}  kbps:{:.0}", fps, kbps);
            if let Ok(json) = pc2.get_stats_json() {
                let (mut jitter, mut rtt, mut lost) = (None, None, None);
                let mut in_inbound = false;
                let mut in_remote = false;
                for line in json.lines() {
                    let t = line.trim();
                    if t.starts_with("InboundRtp") { in_inbound = true; continue; }
                    if t.starts_with("RemoteInboundRtp") { in_inbound = false; in_remote = true; continue; }
                    if t.starts_with("OutboundRtp") || t.starts_with("RemoteOutboundRtp") { in_inbound = false; in_remote = false; continue; }
                    if t == "}" { in_inbound = false; in_remote = false; continue; }
                    if in_inbound && jitter.is_none() && t.starts_with("jitter:") {
                        jitter = t.split(':').nth(1).map(|v| v.trim().trim_end_matches(','));
                    }
                    if in_inbound && lost.is_none() && t.starts_with("packets_lost:") {
                        lost = t.split(':').nth(1).map(|v| v.trim().trim_end_matches(','));
                    }
                    if in_remote && rtt.is_none() && t.starts_with("round_trip_time:") {
                        rtt = t.split(':').nth(1).map(|v| v.trim().trim_end_matches(','));
                    }
                }
                if let Some(v) = jitter { stats.push_str(&format!("  jitter:{}", v)); }
                if let Some(v) = rtt { stats.push_str(&format!("  rtt:{}s", v)); }
                if let Some(v) = lost { stats.push_str(&format!("  lost:{}", v)); }
            }
            *p.receiver_stats.lock().unwrap() = stats;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
