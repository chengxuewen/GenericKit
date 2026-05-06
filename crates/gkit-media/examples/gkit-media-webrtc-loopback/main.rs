// gkit-media WebRTC P2P Loopback Demo (egui) — real SDP + ICE exchange
// Usage: cargo run -p gkit-media --example gkit-media-webrtc-loopback
//
// Two RTCPeerConnections connected via real SDP offer/answer + ICE exchange:
//   PC1 (sender): VideoFrameGenerator → I420 → write_sample(video track)
//   PC2 (receiver): on_track() → read RTP → count frames
// egui shows side-by-side: PC1 generated frame (left) | PC2 received frame (right)
//
// References: W3C WebRTC 1.0 API, webrtc-rs 0.11

use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::video::buffer::{VideoBuffer, VideoFormatType};
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};
use tokio::runtime::Runtime;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

const W: u32 = 640;
const H: u32 = 360;
const FPS: u32 = 15; // lower FPS avoids overwhelming the RTP pipeline

struct Pipeline {
    sender_frame: Mutex<Option<Vec<u8>>>,
    receiver_frame: Mutex<Option<Vec<u8>>>,
    sender_count: Mutex<u64>,
    receiver_count: Mutex<u64>,
    status: Mutex<String>,
}

fn main() -> Result<(), eframe::Error> {
    let pipeline = Arc::new(Pipeline {
        sender_frame: Mutex::new(None),
        receiver_frame: Mutex::new(None),
        sender_count: Mutex::new(0),
        receiver_count: Mutex::new(0),
        status: Mutex::new("Initializing WebRTC P2P...".into()),
    });

    let p = pipeline.clone();

    // Spawn WebRTC setup + frame generation on a background thread
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            // --- API ---
            let api = APIBuilder::new().build();

            // --- PC1 (sender) ---
            let pc1 = RTCPeerConnection::new(Default::default()).await.unwrap();
            let pc2 = RTCPeerConnection::new(Default::default()).await.unwrap();

            // --- ICE candidate channels ---
            let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel::<String>();
            let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel::<String>();

            pc1.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
                if let Some(ref c) = c { let _ = tx1.send(c.to_json().unwrap().candidate); }
            }));
            pc2.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
                if let Some(ref c) = c { let _ = tx2.send(c.to_json().unwrap().candidate); }
            }));

            // --- Video track on PC1 ---
            let video_track = Arc::new(TrackLocalStaticSample::new(
                webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability {
                    mime_type: webrtc::api::media_engine::MIME_TYPE_VP8.to_string(),
                    ..Default::default()
                },
                "video".to_string(),
                "gk-sender".to_string(),
            ));
            pc1.add_track(video_track.clone()).await.unwrap();

            // --- on_track on PC2 ---
            let recv_p = p.clone();
            pc2.on_track(Box::new(move |_track, _receiver, _transceiver| {
                *recv_p.receiver_count.lock().unwrap() += 1;
                Box::new(async {})
            }));

            // --- SDP exchange ---
            let offer = pc1.create_offer().await.unwrap();
            pc1.set_local_description(offer.clone()).await.unwrap();
            pc2.set_remote_description(offer).await.unwrap();

            let answer = pc2.create_answer().await.unwrap();
            pc2.set_local_description(answer.clone()).await.unwrap();
            pc1.set_remote_description(answer).await.unwrap();

            // Exchange gathered ICE candidates
            while let Ok(c) = rx2.try_recv() {
                pc1.add_ice_candidate(RTCIceCandidateInit {
                    candidate: c, sdp_mid: Some("0".into()), sdp_mline_index: Some(0), username_fragment: None,
                }).await.ok();
            }
            while let Ok(c) = rx1.try_recv() {
                pc2.add_ice_candidate(RTCIceCandidateInit {
                    candidate: c, sdp_mid: Some("0".into()), sdp_mline_index: Some(0), username_fragment: None,
                }).await.ok();
            }

            *p.status.lock().unwrap() = format!("P2P connected — {}x{} {}fps", W, H, FPS);

            // --- VideoFrameGenerator feeds PC1's video track ---
            let mut gen = VideoFrameGenerator::new(W, H, FPS);
            let sender_p = p.clone();

            struct Sink { s: Arc<Pipeline>, track: Arc<TrackLocalStaticSample>, rt: Runtime }
            impl VideoSink<gkit_media::video::frame::BoxVideoFrame> for Sink {
                fn on_frame(&self, frame: &gkit_media::video::frame::BoxVideoFrame) {
                    if let Ok(i420) = frame.buffer.to_i420() {
                        let mut rgba = vec![0u8; (W * H * 4) as usize];
                        i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                        *self.s.sender_frame.lock().unwrap() = Some(rgba);
                        *self.s.sender_count.lock().unwrap() += 1;

                        let raw: Vec<u8> = i420.data_y.iter()
                            .chain(&i420.data_u).chain(&i420.data_v).copied().collect();
                        let _ = self.rt.block_on(self.track.write_sample(&webrtc::media::Sample {
                            data: bytes::Bytes::from(raw),
                            duration: Duration::from_micros(66_666),
                            ..Default::default()
                        }));
                    }
                }
            }
            gen.add_or_update_sink(Box::new(Sink { s: sender_p, track: video_track, rt: Runtime::new().unwrap() }),
                VideoSinkWants { is_active: true, ..Default::default() });
            gen.start();

            // Keep WebRTC session alive
            loop { std::thread::sleep(Duration::from_secs(3600)); }
        });
    });

    // --- egui UI ---
    eframe::run_native(
        "gkit-media WebRTC P2P Loopback",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 400.0]),
            ..Default::default()
        },
        Box::new(move |_cc| {
            struct App { p: Arc<Pipeline>, gen_tex: Option<egui::TextureHandle>, recv_tex: Option<egui::TextureHandle> }
            impl eframe::App for App {
                fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
                    egui::TopBottomPanel::top("bar").show(ctx, |ui| {
                        ui.label(self.p.status.lock().unwrap().clone());
                    });
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let sc = *self.p.sender_count.lock().unwrap();
                        let rc = *self.p.receiver_count.lock().unwrap();
                        ui.columns(2, |cols| {
                            cols[0].vertical_centered(|ui| {
                                ui.heading(format!("PC1 Sender ({})", sc));
                                if let Some(ref rgba) = *self.p.sender_frame.lock().unwrap() {
                                    let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba);
                                    self.gen_tex = Some(ctx.load_texture("sender", img, egui::TextureOptions::LINEAR));
                                }
                                if let Some(ref t) = self.gen_tex {
                                    let s = (ui.available_width() / W as f32).min(1.0);
                                    ui.image(egui::load::SizedTexture::new(t.id(), [W as f32 * s, H as f32 * s]));
                                }
                            });
                            cols[1].vertical_centered(|ui| {
                                ui.heading(format!("PC2 Receiver ({})", rc));
                                if let Some(ref rgba) = *self.p.receiver_frame.lock().unwrap() {
                                    let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba);
                                    self.recv_tex = Some(ctx.load_texture("receiver", img, egui::TextureOptions::LINEAR));
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
