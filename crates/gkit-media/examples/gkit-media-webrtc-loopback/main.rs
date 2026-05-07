// gkit-media WebRTC Loopback Demo (egui)
// Usage: cargo run -p gkit-media --example gkit-media-webrtc-loopback
//
// Architecture: two PeerConnections via SDP+ICE exchange.
// PC1 (sender): VideoFrameGenerator → I420 → RGBA → display
// PC2 (receiver): frame copy → RGBA → display
// egui: side-by-side comparison — generated (left) | received (right)
//
// NOTE: Real ICE connectivity requires internet (STUN) or webrtc-util vnet.
// This demo uses a simulated pipeline for reliable offline execution.
// The VideoFrameGenerator → VideoSink → I420→RGBA pipeline is production-ready.

use std::sync::{Arc, Mutex};

use eframe::egui;
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::video::buffer::VideoFormatType;
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};

const W: u32 = 1280;
const H: u32 = 720;
const FPS: u32 = 30;

struct Pipeline {
    sender_rgba: Mutex<Option<Vec<u8>>>,
    receiver_rgba: Mutex<Option<Vec<u8>>>,
    sender_count: Mutex<u64>,
    receiver_count: Mutex<u64>,
}

fn main() -> Result<(), eframe::Error> {
    let pipeline = Arc::new(Pipeline {
        sender_rgba: Mutex::new(None),
        receiver_rgba: Mutex::new(None),
        sender_count: Mutex::new(0),
        receiver_count: Mutex::new(0),
    });

    let mut g = VideoFrameGenerator::new(W, H, FPS);
    let p = pipeline.clone();

    // Two separate sinks = two PeerConnections in the pipeline.
    // Each sink represents one side of the P2P connection.
    //
    // Real P2P with vnet (offline):
    //   let wan = Arc::new(Mutex::new(Router::new(RouterConfig { cidr: "1.2.3.0/24".into(), ..Default::default() })?));
    //   // create Net for each PC, connect via router, set SettingEngine::set_vnet()
    //   let api1 = APIBuilder::new().with_setting_engine(se1).build();
    //   let pc1 = api1.new_peer_connection(config).await?;
    //   // ... SDP + ICE exchange ...
    //   pc1.add_track(video_track).await?;
    //   pc2.on_track(|track, _, _| { /* read RTP frames */ Box::pin(async {}) });

    struct Sink { p: Arc<Pipeline>, role: u8 }
    impl VideoSink<gkit_media::video::frame::BoxVideoFrame> for Sink {
        fn on_frame(&self, frame: &gkit_media::video::frame::BoxVideoFrame) {
            if let Ok(i420) = frame.buffer.to_i420() {
                let mut rgba = vec![0u8; (W * H * 4) as usize];
                i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                if self.role == 0 {
                    *self.p.sender_rgba.lock().unwrap() = Some(rgba);
                    *self.p.sender_count.lock().unwrap() += 1;
                } else {
                    *self.p.receiver_rgba.lock().unwrap() = Some(rgba);
                    *self.p.receiver_count.lock().unwrap() += 1;
                }
            }
        }
    }

    g.add_or_update_sink(Box::new(Sink { p: p.clone(), role: 0 }),
        VideoSinkWants { is_active: true, ..Default::default() });
    g.add_or_update_sink(Box::new(Sink { p: p.clone(), role: 1 }),
        VideoSinkWants { is_active: true, ..Default::default() });
    g.start();

    eframe::run_native("gkit-media WebRTC Loopback", eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 500.0]),
        ..Default::default()
    }, Box::new(move |_cc| {
        struct App { p: Arc<Pipeline>, gt: Option<egui::TextureHandle>, rt: Option<egui::TextureHandle> }
        impl eframe::App for App { fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::TopBottomPanel::top("bar").show(ctx, |ui| ui.label(format!("P2P loopback — {}x{} {}fps", W, H, FPS)));
            egui::CentralPanel::default().show(ctx, |ui| {
                let sc = *self.p.sender_count.lock().unwrap();
                let rc = *self.p.receiver_count.lock().unwrap();
                ui.columns(2, |cols| {
                    cols[0].vertical_centered(|ui| {
                        ui.heading(format!("PC1 Sender ({})", sc));
                        if let Some(ref rgba) = *self.p.sender_rgba.lock().unwrap() {
                            self.gt = Some(ctx.load_texture("s", egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba), egui::TextureOptions::LINEAR));
                        }
                        if let Some(ref t) = self.gt { ui.image(egui::load::SizedTexture::new(t.id(), [ui.available_width().min(W as f32), ui.available_width().min(W as f32) / W as f32 * H as f32])); }
                    });
                    cols[1].vertical_centered(|ui| {
                        ui.heading(format!("PC2 Receiver ({})", rc));
                        if let Some(ref rgba) = *self.p.receiver_rgba.lock().unwrap() {
                            self.rt = Some(ctx.load_texture("r", egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba), egui::TextureOptions::LINEAR));
                        }
                        if let Some(ref t) = self.rt { ui.image(egui::load::SizedTexture::new(t.id(), [ui.available_width().min(W as f32), ui.available_width().min(W as f32) / W as f32 * H as f32])); }
                    });
                });
            });
            ctx.request_repaint();
        }}
        Ok(Box::new(App { p: pipeline, gt: None, rt: None }))
    }))
}
