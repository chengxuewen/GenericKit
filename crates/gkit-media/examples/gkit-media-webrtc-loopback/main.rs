// gkit-media P2P Loopback — tests gkit RTC API
// Usage: cargo run -p gkit-media --example gkit-media-webrtc-loopback --features backend-native-webrtc-rs
//
// Two PeerConnections via gkit-media API with SDP+ICE negotiation.
// VideoFrameGenerator produces frames displayed in egui.
// PC states observed via core::PeerConnection trait methods.

use std::sync::{Arc, Mutex};
use eframe::egui;
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::protocols::rtc::client::core::{PeerConnection, PeerConnectionFactory, IceCandidate};
use gkit_media::protocols::rtc::client::native::NativeFactory;
use gkit_media::video::buffer::VideoFormatType;
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};

const W: u32 = 640; const H: u32 = 360; const FPS: u32 = 15;

struct Pipeline {
    sender_frame: Mutex<Option<Vec<u8>>>,
    sender_count: Mutex<u64>,
    pc1_ice: Mutex<String>, pc2_ice: Mutex<String>,
    status: Mutex<String>,
}

fn main() -> Result<(), eframe::Error> {
    let pipeline = Arc::new(Pipeline {
        sender_frame: Mutex::new(None), sender_count: Mutex::new(0),
        pc1_ice: Mutex::new("—".into()), pc2_ice: Mutex::new("—".into()),
        status: Mutex::new("Creating gkit P2P...".into()),
    });

    // Start frame generator immediately
    let mut g = VideoFrameGenerator::new(W, H, FPS);
    let dp = pipeline.clone();
    struct Sink { p: Arc<Pipeline> }
    impl VideoSink<gkit_media::video::frame::BoxVideoFrame> for Sink {
        fn on_frame(&self, frame: &gkit_media::video::frame::BoxVideoFrame) {
            if let Ok(i420) = frame.buffer.to_i420() {
                let mut rgba = vec![0u8; (W * H * 4) as usize];
                i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                *self.p.sender_frame.lock().unwrap() = Some(rgba);
                *self.p.sender_count.lock().unwrap() += 1;
            }
        }
    }
    g.add_or_update_sink(Box::new(Sink { p: dp }), VideoSinkWants { is_active: true, ..Default::default() });
    g.start();

    // P2P negotiation via gkit API in background thread
    let p = pipeline.clone();
    std::thread::spawn(move || {
        let factory = NativeFactory::default();
        let mut pc1 = factory.create_peer_connection().expect("create pc1");
        let mut pc2 = factory.create_peer_connection().expect("create pc2");

        *p.status.lock().unwrap() = "ICE gathering...".into();

        let (tx1, rx1) = std::sync::mpsc::channel::<IceCandidate>();
        let (tx2, rx2) = std::sync::mpsc::channel::<IceCandidate>();
        pc1.set_on_ice_candidate(Box::new(move |c| { let _ = tx2.send(c); }));
        pc2.set_on_ice_candidate(Box::new(move |c| { let _ = tx1.send(c); }));

        let offer = pc1.create_offer().expect("offer");
        pc1.set_local_description(&offer).expect("set local");
        pc2.set_remote_description(&offer).expect("set remote");

        let answer = pc2.create_answer().expect("answer");
        pc2.set_local_description(&answer).expect("set local");
        pc1.set_remote_description(&answer).expect("set remote");

        pc1.gather_complete().expect("gather1");
        pc2.gather_complete().expect("gather2");

        for c in rx2.try_iter() { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
        for c in rx1.try_iter() { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }

        *p.status.lock().unwrap() = format!("gkit P2P negotiated — {}x{} {}fps", W, H, FPS);

        loop {
            *p.pc1_ice.lock().unwrap() = format!("{:?}", pc1.ice_connection_state());
            *p.pc2_ice.lock().unwrap() = format!("{:?}", pc2.ice_connection_state());
            for c in rx2.try_iter() { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
            for c in rx1.try_iter() { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    eframe::run_native("gkit-media P2P (gkit API)", eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 400.0]), ..Default::default()
    }, Box::new(move |_cc| {
        struct App { p: Arc<Pipeline>, gt: Option<egui::TextureHandle> }
        impl eframe::App for App { fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::TopBottomPanel::top("bar").show(ctx, |ui| ui.label(self.p.status.lock().unwrap().clone()));
            egui::CentralPanel::default().show(ctx, |ui| {
                let sc = *self.p.sender_count.lock().unwrap();
                ui.columns(2, |cols| {
                    cols[0].vertical_centered(|ui| {
                        ui.heading(format!("PC1 Sender ({:?})", sc));
                        if let Some(ref rgba) = *self.p.sender_frame.lock().unwrap() {
                            let img = egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba);
                            self.gt = Some(ctx.load_texture("s", img, egui::TextureOptions::LINEAR));
                        }
                        if let Some(ref t) = self.gt {
                            let s = (ui.available_width() / W as f32).min(1.0);
                            ui.image(egui::load::SizedTexture::new(t.id(), [W as f32 * s, H as f32 * s]));
                        }
                        ui.separator();
                        ui.label(format!("ICE: {}", self.p.pc1_ice.lock().unwrap()));
                    });
                    cols[1].vertical_centered(|ui| {
                        ui.heading("PC2 Receiver");
                        ui.separator();
                        ui.label(format!("ICE: {}", self.p.pc2_ice.lock().unwrap()));
                        ui.label("gkit PeerConnection API drive");
                    });
                });
            });
            ctx.request_repaint();
        }}
        Ok(Box::new(App { p: pipeline, gt: None }))
    }))
}
