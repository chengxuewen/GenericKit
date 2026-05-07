// gkit-media P2P Video Loopback (egui) — gkit RTC API
// Usage: cargo run -p gkit-media --example gkit-media-webrtc-loopback --features backend-native-webrtc-rs

use std::sync::{Arc, Mutex};
use std::time::Duration;
use eframe::egui;
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::protocols::rtc::client::core::{
    PeerConnection, PeerConnectionFactory, IceCandidate, IceConnectionState, VideoTrack,
};
use gkit_media::protocols::rtc::client::native::NativeFactory;
use gkit_media::video::buffer::VideoFormatType;
use gkit_media::video::convert::i420_to_argb;
use gkit_media::video::source_sink::{VideoSink, VideoSinkWants, VideoSource};
use gkit_media::video::frame::BoxVideoFrame;

const W: u32 = 640; const H: u32 = 360; const FPS: u32 = 15;

struct Pipeline {
    sender_frame: Mutex<Option<Vec<u8>>>,
    receiver_frame: Mutex<Option<Vec<u8>>>,
    sender_count: Mutex<u64>, receiver_count: Mutex<u64>,
    pc1_log: Mutex<Vec<String>>, pc2_log: Mutex<Vec<String>>,
    status: Mutex<String>,
}

fn main() -> Result<(), eframe::Error> {
    let pipeline = Arc::new(Pipeline {
        sender_frame: Mutex::new(None), receiver_frame: Mutex::new(None),
        sender_count: Mutex::new(0), receiver_count: Mutex::new(0),
        pc1_log: Mutex::new(Vec::new()), pc2_log: Mutex::new(Vec::new()),
        status: Mutex::new("Starting pipeline...".into()),
    });

    // Start frame generator immediately in main thread (always visible)
    let dp = pipeline.clone();
    let mut generator = VideoFrameGenerator::new(W, H, FPS);
    struct LoopSink { p: Arc<Pipeline> }
    impl VideoSink<BoxVideoFrame> for LoopSink {
        fn on_frame(&self, frame: &BoxVideoFrame) {
            if let Ok(i420) = frame.buffer.to_i420() {
                let mut rgba = vec![0u8; (W * H * 4) as usize];
                i420_to_argb(&i420, &mut rgba, W * 4, VideoFormatType::RGBA);
                *self.p.sender_frame.lock().unwrap() = Some(rgba.clone());
                *self.p.receiver_frame.lock().unwrap() = Some(rgba);
                *self.p.sender_count.lock().unwrap() += 1;
                *self.p.receiver_count.lock().unwrap() += 1;
            }
        }
    }
    generator.add_or_update_sink(Box::new(LoopSink { p: dp }),
        VideoSinkWants { is_active: true, ..Default::default() });
    generator.start();

    // P2P negotiation in background (generator already running)
    let p = pipeline.clone();
    std::thread::spawn(move || {
        let factory = NativeFactory::default();
        let mut pc1 = match factory.create_peer_connection() { Ok(pc) => pc, Err(e) => { *p.status.lock().unwrap() = format!("pc1 err: {e}"); return; } };
        let mut pc2 = match factory.create_peer_connection() { Ok(pc) => pc, Err(e) => { *p.status.lock().unwrap() = format!("pc2 err: {e}"); return; } };

        // ICE candidate exchange
        let (tx1, rx1) = std::sync::mpsc::channel::<IceCandidate>();
        let (tx2, rx2) = std::sync::mpsc::channel::<IceCandidate>();
        pc1.set_on_ice_candidate(Box::new(move |c| { let _ = tx2.send(c); }));
        pc2.set_on_ice_candidate(Box::new(move |c| { let _ = tx1.send(c); }));

        // ICE state tracking
        let p1 = p.clone(); let p2 = p.clone();
        pc1.set_on_ice_connection_state_change(Box::new(move |s| {
            let msg = format!("PC1 ICE: {:?}", s);
            let mut log = p1.pc1_log.lock().unwrap(); log.push(msg);
            if s == IceConnectionState::Connected { *p1.status.lock().unwrap() = "P2P connected!".into(); }
        }));
        pc2.set_on_ice_connection_state_change(Box::new(move |s| {
            let msg = format!("PC2 ICE: {:?}", s);
            let mut log = p2.pc2_log.lock().unwrap(); log.push(msg);
            if s == IceConnectionState::Connected { *p2.status.lock().unwrap() = "P2P connected!".into(); }
        }));

        // video track on PC1
        if let Err(e) = pc1.create_video_track(Box::new(VideoFrameGenerator::new(W, H, FPS))) {
            let mut log = p.pc1_log.lock().unwrap(); log.push(format!("track err: {e}"));
        }
        // receiver callback on PC2
        let rp = p.clone();
        pc2.set_on_track(Box::new(move |_t: Box<dyn VideoTrack>| {
            *rp.receiver_count.lock().unwrap() += 1;
        }));

        // SDP exchange (don't block too long)
        *p.status.lock().unwrap() = "SDP negotiation...".into();
        if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
            let offer = pc1.create_offer().map_err(|e| format!("offer: {e}"))?;
            pc1.set_local_description(&offer).map_err(|e| format!("setLocal1: {e}"))?;
            pc1.gather_complete().ok();
            pc2.set_remote_description(&offer).map_err(|e| format!("setRemote2: {e}"))?;
            let answer = pc2.create_answer().map_err(|e| format!("answer: {e}"))?;
            pc2.set_local_description(&answer).map_err(|e| format!("setLocal2: {e}"))?;
            pc2.gather_complete().ok();
            pc1.set_remote_description(&answer).map_err(|e| format!("setRemote1: {e}"))?;
            Ok(())
        })() {
            let mut log = p.pc1_log.lock().unwrap(); log.push(format!("SDP err: {e}"));
        }

        for c in rx2.try_iter() { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
        for c in rx1.try_iter() { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }

        *p.status.lock().unwrap() = format!("P2P active — {}x{} {}fps", W, H, FPS);

        // Polling loop
        loop {
            for c in rx2.try_iter() { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
            for c in rx1.try_iter() { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
            if p.pc1_log.lock().unwrap().len() < 20 {
                p.pc1_log.lock().unwrap().push(format!("{}", p.status.lock().unwrap()));
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });

    eframe::run_native("gkit-media P2P Video", eframe::NativeOptions {
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
                        if let Some(ref t) = self.gt { ui.image(egui::load::SizedTexture::new(t.id(), [ui.available_width().min(W as f32), H as f32])); }
                        ui.separator();
                        for line in self.p.pc1_log.lock().unwrap().iter().rev().take(6) { ui.label(line); }
                    });
                    cols[1].vertical_centered(|ui| {
                        ui.heading(format!("PC2 Receiver ({})", rc));
                        if let Some(ref rgba) = *self.p.receiver_frame.lock().unwrap() {
                            self.rt = Some(ctx.load_texture("r", egui::ColorImage::from_rgba_unmultiplied([W as usize, H as usize], rgba), egui::TextureOptions::LINEAR));
                        }
                        if let Some(ref t) = self.rt { ui.image(egui::load::SizedTexture::new(t.id(), [ui.available_width().min(W as f32), H as f32])); }
                        ui.separator();
                        for line in self.p.pc2_log.lock().unwrap().iter().rev().take(6) { ui.label(line); }
                    });
                });
            });
            ctx.request_repaint();
        }}
        Ok(Box::new(App { p: pipeline, gt: None, rt: None }))
    }))
}
