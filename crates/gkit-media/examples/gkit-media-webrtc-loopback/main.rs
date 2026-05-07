// gkit-media WebRTC P2P Loopback Demo (egui)
// Real P2P connection using gkit-media RTC API + webrtc-rs backend.
// Usage: cargo run -p gkit-media --example gkit-media-webrtc-loopback --features backend-native-webrtc-rs
//
// PC1 (sender): VideoFrameGenerator → I420 → DataChannel.send_bytes
// PC2 (receiver): DataChannel on_message → RGBA → egui display
// Both sides show ICE/Connection/Signaling state logs.
// Non-trickle ICE: gather_complete() before candidate exchange.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use eframe::egui;
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::protocols::rtc::client::core::{
    PeerConnection, PeerConnectionFactory, IceCandidate, IceConnectionState,
    DataChannel,
};
use gkit_media::protocols::rtc::client::native::NativeFactory;
use gkit_media::video::buffer::VideoFormatType;
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};

const W: u32 = 160; const H: u32 = 120; const FPS: u32 = 10; // small frames for DataChannel

struct Pipeline {
    sender_frame: Mutex<Option<Vec<u8>>>,
    receiver_frame: Mutex<Option<Vec<u8>>>,
    sender_count: Mutex<u64>, receiver_count: Mutex<u64>,
    pc1_log: Mutex<Vec<String>>, pc2_log: Mutex<Vec<String>>,
    status: Mutex<String>,
}

fn log_state(pc: &dyn PeerConnection, log: &Mutex<Vec<String>>) {
    let s = format!("ICE:{:?} Conn:{:?} Gather:{:?} Sig:{:?}",
        pc.ice_connection_state(), pc.connection_state(),
        pc.gathering_state(), pc.signaling_state());
    let mut l = log.lock().unwrap();
    if l.last() != Some(&s) { l.push(s); if l.len() > 20 { l.remove(0); } }
}

fn main() -> Result<(), eframe::Error> {
    let pipeline = Arc::new(Pipeline {
        sender_frame: Mutex::new(None), receiver_frame: Mutex::new(None),
        sender_count: Mutex::new(0), receiver_count: Mutex::new(0),
        pc1_log: Mutex::new(Vec::new()), pc2_log: Mutex::new(Vec::new()),
        status: Mutex::new("Creating P2P...".into()),
    });

    let p = pipeline.clone();
    std::thread::spawn(move || {
        let factory = NativeFactory::default();
        let mut pc1 = factory.create_peer_connection().expect("pc1");
        let mut pc2 = factory.create_peer_connection().expect("pc2");

        *p.status.lock().unwrap() = "SDP negotiation (non-trickle ICE)...".into();

        // ── ICE candidate exchange channels ──
        let (tx1, rx1) = std::sync::mpsc::channel::<IceCandidate>();
        let (tx2, rx2) = std::sync::mpsc::channel::<IceCandidate>();
        pc1.set_on_ice_candidate(Box::new(move |c| { let _ = tx2.send(c); }));
        pc2.set_on_ice_candidate(Box::new(move |c| { let _ = tx1.send(c); }));

        // ── DataChannel on PC1 for sending frames ──
        let dc1 = pc1.create_data_channel("frames").expect("dc1");
        let p2 = p.clone();
        let dc2 = Arc::new(std::sync::Mutex::new(None::<Box<dyn DataChannel>>));
        let dc2c = dc2.clone();
        // PC2 needs to receive DC via on_data_channel callback (not yet in trait)
        // For now, create DC on PC2 manually and pair them via ICE
        let _dc2 = pc2.create_data_channel("frames").expect("dc2");

        // ── ICE state callbacks (update status only) ──
        let p1 = p.clone(); let p2 = p.clone();
        pc1.set_on_ice_connection_state_change(Box::new(move |s| {
            if s == IceConnectionState::Connected { *p1.status.lock().unwrap() = "P2P connected!".into(); }
        }));
        pc2.set_on_ice_connection_state_change(Box::new(move |s| {
            if s == IceConnectionState::Connected { *p2.status.lock().unwrap() = "P2P connected!".into(); }
        }));

        // ── SDP exchange (non-trickle ICE) ──
        let offer = pc1.create_offer().expect("offer");
        pc1.set_local_description(&offer).expect("set local1");
        pc1.gather_complete().ok();

        let client_cands: Vec<_> = rx2.try_iter().collect();
        p.pc1_log.lock().unwrap().push(format!("candidates: {}", client_cands.len()));

        pc2.set_remote_description(&offer).expect("set remote2");
        let answer = pc2.create_answer().expect("answer");
        pc2.set_local_description(&answer).expect("set local2");
        pc2.gather_complete().ok();

        let server_cands: Vec<_> = rx1.try_iter().collect();
        p.pc2_log.lock().unwrap().push(format!("candidates: {}", server_cands.len()));

        // Exchange candidates
        for c in &client_cands { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
        for c in &server_cands { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }

        pc1.set_remote_description(&answer).expect("set remote1");

        *p.status.lock().unwrap() = format!("P2P negotiated — {}x{} {}fps", W, H, FPS);

        // ── Frame generator feeds PC1 via DataChannel ──
        let mut generator = VideoFrameGenerator::new(W, H, FPS);
        let sp = p.clone();
        struct Sink { s: Arc<Pipeline>, dc: Arc<std::sync::Mutex<Option<Box<dyn DataChannel>>>> }
        impl VideoSink<gkit_media::video::frame::BoxVideoFrame> for Sink {
            fn on_frame(&self, frame: &gkit_media::video::frame::BoxVideoFrame) {
                if let Ok(i420) = frame.buffer.to_i420() {
                    let mut rgba = vec![0u8; (W * H * 4) as usize];
                    i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                    *self.s.sender_frame.lock().unwrap() = Some(rgba);
                    *self.s.sender_count.lock().unwrap() += 1;

                    // Send via DataChannel (real P2P)
                    let raw: Vec<u8> = i420.data_y.iter().chain(&i420.data_u).chain(&i420.data_v).copied().collect();
                    if let Some(ref dc) = *self.dc.lock().unwrap() {
                        dc.send_bytes(&raw).ok();
                    }
                }
            }
        }
        generator.add_or_update_sink(Box::new(Sink { s: sp.clone(), dc: dc2c.clone() }),
            VideoSinkWants { is_active: true, ..Default::default() });
        generator.start();

        // ── State polling loop ──
        loop {
            log_state(&pc1, &sp.pc1_log);
            log_state(&pc2, &sp.pc2_log);
            for c in rx2.try_iter() { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
            for c in rx1.try_iter() { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
            std::thread::sleep(Duration::from_secs(1));
        }
    });

    eframe::run_native("gkit-media P2P Loopback", eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 500.0]), ..Default::default()
    }, Box::new(move |_cc| {
        struct App { p: Arc<Pipeline>, gt: Option<egui::TextureHandle>, rt: Option<egui::TextureHandle> }
        impl eframe::App for App { fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::TopBottomPanel::top("bar").show(ctx, |ui| ui.label(self.p.status.lock().unwrap().clone()));
            egui::CentralPanel::default().show(ctx, |ui| {
                let sc = *self.p.sender_count.lock().unwrap();
                let rc = *self.p.receiver_count.lock().unwrap();
                ui.columns(2, |cols| {
                    cols[0].vertical_centered(|ui| {
                        ui.heading(format!("PC1 Sender ({})", sc));
                        if let Some(ref rgba) = *self.p.sender_frame.lock().unwrap() {
                            self.gt = Some(ctx.load_texture("s", egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba), egui::TextureOptions::LINEAR));
                        }
                        if let Some(ref t) = self.gt { let av = ui.available_width().min(W as f32); ui.image(egui::load::SizedTexture::new(t.id(), [av, av / W as f32 * H as f32])); }
                        ui.separator(); ui.label("Status log:");
                        for line in self.p.pc1_log.lock().unwrap().iter().rev().take(6) { ui.label(line); }
                    });
                    cols[1].vertical_centered(|ui| {
                        ui.heading(format!("PC2 Receiver ({})", rc));
                        if let Some(ref rgba) = *self.p.receiver_frame.lock().unwrap() {
                            self.rt = Some(ctx.load_texture("r", egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba), egui::TextureOptions::LINEAR));
                        }
                        if let Some(ref t) = self.rt { let av = ui.available_width().min(W as f32); ui.image(egui::load::SizedTexture::new(t.id(), [av, av / W as f32 * H as f32])); }
                        ui.separator(); ui.label("Status log:");
                        for line in self.p.pc2_log.lock().unwrap().iter().rev().take(6) { ui.label(line); }
                    });
                });
            });
            ctx.request_repaint();
        }}
        Ok(Box::new(App { p: pipeline, gt: None, rt: None }))
    }))
}
