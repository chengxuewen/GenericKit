// gkit-media P2P Video Loopback (egui) — plugin-based backend discovery
// Usage:
//   cargo build -p gkit-plugin-webrtc-libwebrtc  # build plugin dylib first
//   cargo run -p gkit-media --example gkit-media-webrtc-loopback
//
// Backends are discovered dynamically from target/debug/plugins/ via RtcEngine::load_plugins().

use std::sync::{Arc, Mutex};
use std::time::Duration;
use eframe::egui;
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::protocols::rtc::client::core::{
    PeerConnection, IceCandidate, IceConnectionState, VideoTrack,
    SessionDescription, RtcConfiguration, IceServer,
};
use gkit_media::protocols::rtc::client::engine::RtcEngine;
use gkit_media::video::buffer::VideoFormatType;
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};
use gkit_media::video::frame::BoxVideoFrame;

const W: u32 = 640;
const H: u32 = 360;
const FPS: u32 = 15;

#[derive(Debug)]
enum P2pState {
    Idle,
    Connecting,
    Connected,
    Error(String),
}

struct Pipeline {
    sender_frame: Mutex<Option<Vec<u8>>>,
    receiver_frame: Mutex<Option<Vec<u8>>>,
    sender_count: Mutex<u64>,
    receiver_count: Mutex<u64>,
    pc1_log: Mutex<Vec<String>>,
    pc2_log: Mutex<Vec<String>>,
    status: Mutex<String>,
    p2p_state: Mutex<P2pState>,
    selected_backend: Mutex<String>,
    available_backends: Mutex<Vec<String>>,
    ice_config: RtcConfiguration,
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
    RtcEngine::load_plugins();
    let backends = RtcEngine::registered_types();
    let default_backend = if backends.contains(&"libwebrtc".to_string()) {
        "libwebrtc"
    } else if backends.contains(&"webrtc-rs".to_string()) {
        "webrtc-rs"
    } else {
        backends.first().map(|s| s.as_str()).unwrap_or("none")
    };

    let pipeline = Arc::new(Pipeline {
        sender_frame: Mutex::new(None),
        receiver_frame: Mutex::new(None),
        sender_count: Mutex::new(0),
        receiver_count: Mutex::new(0),
        pc1_log: Mutex::new(Vec::new()),
        pc2_log: Mutex::new(Vec::new()),
        status: Mutex::new(format!("Select backend and press Start")),
        p2p_state: Mutex::new(P2pState::Idle),
        selected_backend: Mutex::new(default_backend.to_string()),
        available_backends: Mutex::new(backends),
        ice_config: default_ice_config(),
    });

    // Frame generator — always running
    let dp = pipeline.clone();
    let mut generator = VideoFrameGenerator::new(W, H, FPS);
    struct LoopSink {
        p: Arc<Pipeline>,
    }
    impl VideoSink<BoxVideoFrame> for LoopSink {
        fn on_frame(&self, frame: &BoxVideoFrame) {
            if let Ok(i420) = frame.buffer.to_i420() {
                let mut rgba = vec![0u8; (W * H * 4) as usize];
                i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                *self.p.sender_frame.lock().unwrap() = Some(rgba);
                *self.p.sender_count.lock().unwrap() += 1;
            }
        }
    }
    generator.add_or_update_sink(
        Box::new(LoopSink { p: dp }),
        VideoSinkWants {
            is_active: true,
            ..Default::default()
        },
    );
    generator.start();

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
                                    p.pc1_log.lock().unwrap().clear();
                                    p.pc2_log.lock().unwrap().clear();
                                    std::thread::spawn(move || run_p2p(p, backend));
                                }
                            });

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
                                if let Some(ref rgba) = *self.p.sender_frame.lock().unwrap() {
                                    self.gt = Some(ctx.load_texture(
                                        "s",
                                        egui::ColorImage::from_rgba_unmultiplied(
                                            [W as usize, H as usize],
                                            rgba,
                                        ),
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                                if let Some(ref t) = self.gt {
                                    ui.image(egui::load::SizedTexture::new(
                                        t.id(),
                                        [ui.available_width().min(W as f32), H as f32],
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
                                if let Some(ref rgba) = *self.p.receiver_frame.lock().unwrap() {
                                    self.rt = Some(ctx.load_texture(
                                        "r",
                                        egui::ColorImage::from_rgba_unmultiplied(
                                            [W as usize, H as usize],
                                            rgba,
                                        ),
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                                if let Some(ref t) = self.rt {
                                    ui.image(egui::load::SizedTexture::new(
                                        t.id(),
                                        [ui.available_width().min(W as f32), H as f32],
                                    ));
                                }
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

fn run_p2p(p: Arc<Pipeline>, backend: String) {
    let log = |peer: &str, msg: &str| {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() % 100000)
            .unwrap_or(0);
        let line = format!("[{:05}] {}: {}", ts, peer, msg);
        eprintln!("{}", line);
        match peer {
            "PC1" => p.pc1_log.lock().unwrap().push(line),
            "PC2" => p.pc2_log.lock().unwrap().push(line),
            _ => {}
        }
    };

    // Create factory via RtcEngine
    let factory = match RtcEngine::create(&backend) {
        Ok(f) => {
            log("SYS", &format!("Backend '{}' loaded", backend));
            f
        }
        Err(e) => {
            *p.status.lock().unwrap() = format!("Error: {}", e);
            *p.p2p_state.lock().unwrap() = P2pState::Error(format!("{}", e));
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
            *p.p2p_state.lock().unwrap() = P2pState::Error(format!("{}", e));
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
            *p.p2p_state.lock().unwrap() = P2pState::Error(format!("{}", e));
            pc1.close().ok();
            return;
        }
    };

    // --- ICE candidate relay ---
    let (tx1, rx1) = std::sync::mpsc::channel::<IceCandidate>();
    let (tx2, rx2) = std::sync::mpsc::channel::<IceCandidate>();
    pc1.set_on_ice_candidate(Box::new(move |c| {
        let _ = tx2.send(c);
    }));
    pc2.set_on_ice_candidate(Box::new(move |c| {
        let _ = tx1.send(c);
    }));

    // --- ICE state tracking ---
    {
        let p1 = p.clone();
        pc1.set_on_ice_connection_state_change(Box::new(move |s| {
            p1.pc1_log
                .lock()
                .unwrap()
                .push(format!("ICE state: {:?}", s));
            if s == IceConnectionState::Connected {
                *p1.status.lock().unwrap() = "P2P Connected!".into();
                *p1.p2p_state.lock().unwrap() = P2pState::Connected;
            }
        }));
    }
    {
        let p2 = p.clone();
        pc2.set_on_ice_connection_state_change(Box::new(move |s| {
            p2.pc2_log
                .lock()
                .unwrap()
                .push(format!("ICE state: {:?}", s));
            if s == IceConnectionState::Connected {
                *p2.status.lock().unwrap() = "P2P Connected!".into();
                *p2.p2p_state.lock().unwrap() = P2pState::Connected;
            }
        }));
    }

    // --- Video track on PC1 ---
    let mut track_gen = VideoFrameGenerator::new(W, H, FPS);
    track_gen.start();
    match pc1.create_video_track(Box::new(track_gen)) {
        Ok(track) => {
            log("PC1", &format!("VideoTrack added: {}", track.id()));
        }
        Err(e) => {
            log("PC1", &format!("VideoTrack error: {}", e));
        }
    }

    // --- Receiver on PC2 ---
    let rp = p.clone();
    pc2.set_on_track(Box::new(move |track: Box<dyn VideoTrack>| {
        let dp = rp.clone();
        struct P2PSink {
            p: Arc<Pipeline>,
        }
        impl VideoSink<BoxVideoFrame> for P2PSink {
            fn on_frame(&self, frame: &BoxVideoFrame) {
                if let Ok(i420) = frame.buffer.to_i420() {
                    let mut rgba = vec![0u8; (W * H * 4) as usize];
                    i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                    *self.p.receiver_frame.lock().unwrap() = Some(rgba);
                    *self.p.receiver_count.lock().unwrap() += 1;
                }
            }
        }
        track.add_sink(Box::new(P2PSink { p: dp }));
    }));

    // --- SDP exchange ---
    *p.status.lock().unwrap() = "SDP negotiation...".into();
    if let Err(e) = (|| -> Result<(), String> {
        let offer = pc1.create_offer().map_err(|e| format!("offer: {e}"))?;
        log("PC1", &format!("Offer SDP ({} lines)", offer.sdp.lines().count()));
        pc1.set_local_description(&offer).map_err(|e| format!("setLocal1: {e}"))?;

        pc2.set_remote_description(&offer).map_err(|e| format!("setRemote2: {e}"))?;
        let answer = pc2.create_answer().map_err(|e| format!("answer: {e}"))?;
        log("PC2", &format!("Answer SDP ({} lines)", answer.sdp.lines().count()));
        pc2.set_local_description(&answer).map_err(|e| format!("setLocal2: {e}"))?;

        pc1.set_remote_description(&answer).map_err(|e| format!("setRemote1: {e}"))?;
        Ok(())
    })() {
        log("SDP", &format!("Error: {}", e));
        *p.p2p_state.lock().unwrap() = P2pState::Error(e);
        pc1.close().ok();
        pc2.close().ok();
        return;
    }

    // --- ICE candidate exchange loop ---
    *p.status.lock().unwrap() = format!("P2P active — {}x{} @ {}fps", W, H, FPS);
    let start = std::time::Instant::now();
    loop {
        // Relay ICE candidates
        for c in rx2.try_iter() {
            pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok();
            log("ICE", &format!("PC1→PC2 candidate: mid={:?}", c.sdp_mid));
        }
        for c in rx1.try_iter() {
            pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok();
            log("ICE", &format!("PC2→PC1 candidate: mid={:?}", c.sdp_mid));
        }

        // Timeout after 60s
        if start.elapsed() > Duration::from_secs(60)
            && matches!(*p.p2p_state.lock().unwrap(), P2pState::Connecting)
        {
            *p.status.lock().unwrap() = "ICE timeout (60s)".into();
            *p.p2p_state.lock().unwrap() = P2pState::Error("ICE timeout".into());
            pc1.close().ok();
            pc2.close().ok();
            break;
        }

        if matches!(*p.p2p_state.lock().unwrap(), P2pState::Connected | P2pState::Error(_)) {
            break;
        }

        std::thread::sleep(Duration::from_millis(500));
    }
}
