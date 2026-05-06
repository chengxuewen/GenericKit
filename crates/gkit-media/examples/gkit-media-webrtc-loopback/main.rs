// gkit-media WebRTC Loopback Demo (egui) — simulated pipeline version
// Usage: cargo run -p gkit-media --example gkit-media-webrtc-loopback
//
// Simulates two PeerConnections connected via SDP/ICE exchange.
// PC1 (sender): VideoFrameGenerator → I420 → RGBA → display
// PC2 (receiver): receives frame copy → RGBA → display
// egui shows side-by-side: generated frame (left) | received frame (right)
//
// In production, the simulated queue would be replaced by:
//   webrtc::RTCPeerConnection with SDP offer/answer + ICE candidate exchange
//   PC1: add_track(video_track) → write_sample per frame
//   PC2: on_track() → read_rtp → decode → RGBA

use std::sync::{Arc, Mutex};

use eframe::egui;
use gkit_media::video::buffer::VideoFormatType;
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};
use gkit_media::capture::generator::VideoFrameGenerator;

const W: u32 = 1280;
const H: u32 = 720;
const FPS: u32 = 30;

struct Pipeline {
    generated_rgba: Mutex<Option<Vec<u8>>>,
    received_rgba: Mutex<Option<Vec<u8>>>,
    gen_count: Mutex<u64>,
    recv_count: Mutex<u64>,
}

fn main() -> Result<(), eframe::Error> {
    let pipeline = Arc::new(Pipeline {
        generated_rgba: Mutex::new(None),
        received_rgba: Mutex::new(None),
        gen_count: Mutex::new(0),
        recv_count: Mutex::new(0),
    });

    let mut g = VideoFrameGenerator::new(W, H, FPS);
    let p = pipeline.clone();

    struct SimSink { p: Arc<Pipeline>, is_sender: bool }
    impl VideoSink<gkit_media::video::frame::BoxVideoFrame> for SimSink {
        fn on_frame(&self, frame: &gkit_media::video::frame::BoxVideoFrame) {
            if let Ok(i420) = frame.buffer.to_i420() {
                let mut rgba = vec![0u8; (W * H * 4) as usize];
                i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                if self.is_sender {
                    *self.p.generated_rgba.lock().unwrap() = Some(rgba);
                    *self.p.gen_count.lock().unwrap() += 1;
                } else {
                    *self.p.received_rgba.lock().unwrap() = Some(rgba);
                    *self.p.recv_count.lock().unwrap() += 1;
                }
            }
        }
    }

    g.add_or_update_sink(Box::new(SimSink { p: p.clone(), is_sender: true }),
        VideoSinkWants { is_active: true, ..Default::default() });
    g.add_or_update_sink(Box::new(SimSink { p: p.clone(), is_sender: false }),
        VideoSinkWants { is_active: true, ..Default::default() });
    g.start();

    eframe::run_native(
        "gkit-media WebRTC Loopback Demo (simulated)",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 500.0]),
            ..Default::default()
        },
        Box::new(move |_cc| {
            struct App { p: Arc<Pipeline>, gen_tex: Option<egui::TextureHandle>, recv_tex: Option<egui::TextureHandle> }
            impl eframe::App for App {
                fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
                    egui::TopBottomPanel::top("status").show(ctx, |ui| {
                        ui.label(format!("Simulated WebRTC loopback — {}x{} {}fps", W, H, FPS));
                    });
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let gc = *self.p.gen_count.lock().unwrap();
                        let rc = *self.p.recv_count.lock().unwrap();
                        ui.columns(2, |cols| {
                            // Left: PC1 Sender (generated)
                            cols[0].vertical_centered(|ui| {
                                ui.heading(format!("PC1 Sender ({})", gc));
                                if let Some(ref rgba) = *self.p.generated_rgba.lock().unwrap() {
                                    let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba);
                                    self.gen_tex = Some(ctx.load_texture("gen", img, egui::TextureOptions::LINEAR));
                                }
                                if let Some(ref t) = self.gen_tex {
                                    let avail = ui.available_width();
                                    let s = (avail / W as f32).min(1.0);
                                    ui.image(egui::load::SizedTexture::new(t.id(), [W as f32 * s, H as f32 * s]));
                                }
                            });
                            // Right: PC2 Receiver (received)
                            cols[1].vertical_centered(|ui| {
                                ui.heading(format!("PC2 Receiver ({})", rc));
                                if let Some(ref rgba) = *self.p.received_rgba.lock().unwrap() {
                                    let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba);
                                    self.recv_tex = Some(ctx.load_texture("recv", img, egui::TextureOptions::LINEAR));
                                }
                                if let Some(ref t) = self.recv_tex {
                                    let avail = ui.available_width();
                                    let s = (avail / W as f32).min(1.0);
                                    ui.image(egui::load::SizedTexture::new(t.id(), [W as f32 * s, H as f32 * s]));
                                }
                            });
                        });
                    });
                    ctx.request_repaint();
                }
            }
            Ok(Box::new(App { p: pipeline, gen_tex: None, recv_tex: None }))
        }),
    )
}
