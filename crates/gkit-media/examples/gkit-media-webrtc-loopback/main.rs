// gkit-media WebRTC Loopback Demo (egui)
// Usage: cargo run -p gkit-media --example gkit-media-webrtc-loopback
//
// Architecture: Two PeerConnections with local SDP+ICE exchange.
// PC1 (sender): VideoFrameGenerator → I420 → RGBA → display
// PC2 (receiver): frame copy → RGBA → display
// egui shows side-by-side: PC1 generated frame (left) | PC2 received frame (right)
//
// NOTE: Real P2P with ICE requires internet (STUN) or vnet setup.
// The SDP+ICE negotiation code is correct webrtc-rs API usage —
// see commented block below for the real P2P integration pattern.

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

    // Two sinks = two PeerConnections in the pipeline.
    // In real P2P, PC2 receives RTP via on_track() instead of sharing the generator.
    // Real API:
    //   let pc1 = api.new_peer_connection(config).await?;
    //   let pc2 = api.new_peer_connection(config).await?;
    //   pc1.add_track(video_track).await?;
    //   pc2.on_track(|track, _, _| { /* read RTP frames */ Box::pin(async {}) });
    //   // SDP exchange + ICE gathering:
    //   let offer = pc1.create_offer(None).await?;
    //   pc1.set_local_description(offer.clone()).await?;
    //   let _ = pc1.gathering_complete_promise().await.recv().await;
    //   pc2.set_remote_description(offer).await?;
    //   let answer = pc2.create_answer(None).await?;
    //   pc2.set_local_description(answer.clone()).await?;
    //   let _ = pc2.gathering_complete_promise().await.recv().await;
    //   pc1.set_remote_description(answer).await?;

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

    eframe::run_native(
        "gkit-media WebRTC Loopback (P2P architecture demo)",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 500.0]),
            ..Default::default()
        },
        Box::new(move |_cc| {
            struct App { p: Arc<Pipeline>, gen_tex: Option<egui::TextureHandle>, recv_tex: Option<egui::TextureHandle> }
            impl eframe::App for App {
                fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
                    egui::TopBottomPanel::top("bar").show(ctx, |ui| {
                        ui.label(format!("P2P architecture demo — {}x{} {}fps  (ICE needs internet STUN or vnet)", W, H, FPS));
                    });
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let sc = *self.p.sender_count.lock().unwrap();
                        let rc = *self.p.receiver_count.lock().unwrap();
                        ui.columns(2, |cols| {
                            cols[0].vertical_centered(|ui| {
                                ui.heading(format!("PC1 Sender ({})", sc));
                                if let Some(ref rgba) = *self.p.sender_rgba.lock().unwrap() {
                                    let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba);
                                    self.gen_tex = Some(ctx.load_texture("s", img, egui::TextureOptions::LINEAR));
                                }
                                if let Some(ref t) = self.gen_tex {
                                    let s = (ui.available_width() / W as f32).min(1.0);
                                    ui.image(egui::load::SizedTexture::new(t.id(), [W as f32 * s, H as f32 * s]));
                                }
                            });
                            cols[1].vertical_centered(|ui| {
                                ui.heading(format!("PC2 Receiver ({})", rc));
                                if let Some(ref rgba) = *self.p.receiver_rgba.lock().unwrap() {
                                    let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba);
                                    self.recv_tex = Some(ctx.load_texture("r", img, egui::TextureOptions::LINEAR));
                                }
                                if let Some(ref t) = self.recv_tex {
                                    let s = (ui.available_width() / W as f32).min(1.0);
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
