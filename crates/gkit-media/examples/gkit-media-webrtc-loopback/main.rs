// gkit-media WebRTC P2P Loopback Demo (egui) — real ICE over STUN
// Usage: cargo run -p gkit-media --example gkit-media-webrtc-loopback
//
// Two RTCPeerConnections via SDP+ICE with Google/Cloudflare STUN servers.
// Requires internet. Both PCs connect over loopback with STUN-discovered addresses.
// PC1 (sender): VideoFrameGenerator → I420 → write_sample(video track)
// PC2 (receiver): on_track() → count RTP frames
// egui: side-by-side — generated frame (left) | receiver frame count (right)

use std::sync::{Arc, Mutex};
use std::time::Duration;
use eframe::egui;
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::video::buffer::VideoFormatType;
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};
use tokio::runtime::Runtime;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

const W: u32 = 640;
const H: u32 = 360;
const FPS: u32 = 15;

fn stun_config() -> RTCConfiguration {
    RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec![
                "stun:stun.l.google.com:19302".into(),
                "stun:stun1.l.google.com:19302".into(),
                "stun:stun2.l.google.com:19302".into(),
                "stun:stun.cloudflare.com:3478".into(),
                "stun:stun.stunprotocol.org:3478".into(),
            ],
            ..Default::default()
        }],
        ..Default::default()
    }
}

struct Pipeline {
    sender_frame: Mutex<Option<Vec<u8>>>,
    sender_count: Mutex<u64>,
    receiver_count: Mutex<u64>,
    status: Mutex<String>,
}

fn main() -> Result<(), eframe::Error> {
    let pipeline = Arc::new(Pipeline {
        sender_frame: Mutex::new(None),
        sender_count: Mutex::new(0),
        receiver_count: Mutex::new(0),
        status: Mutex::new("Creating P2P via STUN...".into()),
    });

    let p = pipeline.clone();
    std::thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            let api = APIBuilder::new().build();
            let config = stun_config();
            let pc1 = Arc::new(api.new_peer_connection(config.clone()).await.unwrap());
            let pc2 = Arc::new(api.new_peer_connection(config).await.unwrap());

            let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel::<RTCIceCandidateInit>();
            let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel::<RTCIceCandidateInit>();

            pc1.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
                let tx = tx1.clone();
                Box::pin(async move { if let Some(c) = c { let _ = tx.send(c.to_json().unwrap()); } })
            }));
            pc2.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
                let tx = tx2.clone();
                Box::pin(async move { if let Some(c) = c { let _ = tx.send(c.to_json().unwrap()); } })
            }));

            let video_track = Arc::new(TrackLocalStaticSample::new(
                webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability {
                    mime_type: webrtc::api::media_engine::MIME_TYPE_VP8.to_string(),
                    ..Default::default()
                }, "video".into(), "sender".into(),
            ));
            pc1.add_track(video_track.clone()).await.unwrap();

            let rp = p.clone();
            pc2.on_track(Box::new(move |_track, _receiver, _transceiver| {
                *rp.receiver_count.lock().unwrap() += 1;
                Box::pin(async {})
            }));

            *p.status.lock().unwrap() = "SDP+ICE negotiation...".into();
            let offer = pc1.create_offer(None).await.unwrap();
            pc1.set_local_description(offer.clone()).await.unwrap();
            let mut g1 = pc1.gathering_complete_promise().await;
            let _ = g1.recv().await;
            pc2.set_remote_description(offer).await.unwrap();
            let answer = pc2.create_answer(None).await.unwrap();
            pc2.set_local_description(answer.clone()).await.unwrap();
            let mut g2 = pc2.gathering_complete_promise().await;
            let _ = g2.recv().await;
            pc1.set_remote_description(answer).await.unwrap();
            while let Ok(c) = rx2.try_recv() { pc1.add_ice_candidate(c).await.ok(); }
            while let Ok(c) = rx1.try_recv() { pc2.add_ice_candidate(c).await.ok(); }
            let pc1c = pc1.clone(); let pc2c = pc2.clone();
            tokio::spawn(async move { while let Some(c) = rx2.recv().await { pc1c.add_ice_candidate(c).await.ok(); } });
            tokio::spawn(async move { while let Some(c) = rx1.recv().await { pc2c.add_ice_candidate(c).await.ok(); } });
            *p.status.lock().unwrap() = format!("P2P via STUN — {}x{} {}fps", W, H, FPS);

            let mut g = VideoFrameGenerator::new(W, H, FPS);
            let sp = p.clone();
            struct Sink { s: Arc<Pipeline>, track: Arc<TrackLocalStaticSample> }
            impl VideoSink<gkit_media::video::frame::BoxVideoFrame> for Sink {
                fn on_frame(&self, frame: &gkit_media::video::frame::BoxVideoFrame) {
                    if let Ok(i420) = frame.buffer.to_i420() {
                        let mut rgba = vec![0u8; (W * H * 4) as usize];
                        i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                        *self.s.sender_frame.lock().unwrap() = Some(rgba);
                        *self.s.sender_count.lock().unwrap() += 1;
                        let raw: Vec<u8> = i420.data_y.iter().chain(&i420.data_u).chain(&i420.data_v).copied().collect();
                        let rt2 = Runtime::new().unwrap();
                        let _ = rt2.block_on(self.track.write_sample(&webrtc::media::Sample {
                            data: bytes::Bytes::from(raw), duration: Duration::from_micros(66_666),
                            ..Default::default()
                        }));
                    }
                }
            }
            g.add_or_update_sink(Box::new(Sink { s: sp, track: video_track }),
                VideoSinkWants { is_active: true, ..Default::default() });
            g.start();
            loop { std::thread::sleep(Duration::from_secs(3600)); }
        });
    });

    eframe::run_native("gkit-media WebRTC P2P (STUN)", eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 400.0]),
        ..Default::default()
    }, Box::new(move |_cc| {
        struct App { p: Arc<Pipeline>, gt: Option<egui::TextureHandle> }
        impl eframe::App for App { fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::TopBottomPanel::top("bar").show(ctx, |ui| ui.label(self.p.status.lock().unwrap().clone()));
            egui::CentralPanel::default().show(ctx, |ui| {
                let sc = *self.p.sender_count.lock().unwrap();
                let rc = *self.p.receiver_count.lock().unwrap();
                ui.columns(2, |cols| {
                    cols[0].vertical_centered(|ui| {
                        ui.heading(format!("PC1 Sender ({})", sc));
                        if let Some(ref rgba) = *self.p.sender_frame.lock().unwrap() {
                            let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba);
                            self.gt = Some(ctx.load_texture("s", img, egui::TextureOptions::LINEAR));
                        }
                        if let Some(ref t) = self.gt {
                            let s = (ui.available_width() / W as f32).min(1.0);
                            ui.image(egui::load::SizedTexture::new(t.id(), [W as f32 * s, H as f32 * s]));
                        }
                    });
                    cols[1].vertical_centered(|ui| {
                        ui.heading(format!("PC2 Receiver ({})", rc));
                        ui.label("RTP frames received; VP8 decode pending");
                    });
                });
            });
            ctx.request_repaint();
        }}
        Ok(Box::new(App { p: pipeline, gt: None }))
    }))
}
