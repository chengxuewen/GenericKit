// W3C WebRTC P2P Connection Test — vnet (offline)
// cargo test -p gkit-media --features backend-native-webrtc-rs --test webrtc_p2p_conn -- --nocapture

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use gkit_media::capture::generator::VideoFrameGenerator;
use gkit_media::protocols::rtc::client::core::{PeerConnection, IceCandidate, IceConnectionState, VideoTrack};
use gkit_media::video::source_sink::{VideoSink, VideoSource};
use gkit_media::protocols::rtc::client::native::NativePeerConnection;
use webrtc::api::setting_engine::SettingEngine;
use webrtc_util::vnet::net::{Net, NetConfig};
use webrtc_util::vnet::router::{Router, RouterConfig};

const W: u32 = 320; const H: u32 = 240; const FPS: u32 = 15;

#[test]
fn p2p_vnet() {
    // Use the same tokio runtime as NativePeerConnection (global singleton)
    let rt = gkit_media::protocols::rtc::client::native::rt();
    let (se1, se2, _wan) = rt.block_on(async {
        let wan = Arc::new(tokio::sync::Mutex::new(Router::new(RouterConfig { cidr: "1.2.3.0/24".to_string(), ..Default::default() }).unwrap()));
        let net1 = Arc::new(Net::new(Some(NetConfig { static_ips: vec!["1.2.3.4".to_string()], ..Default::default() })));
        let net2 = Arc::new(Net::new(Some(NetConfig { static_ips: vec!["1.2.3.5".to_string()], ..Default::default() })));
        let nic1 = net1.get_nic().unwrap(); let nic2 = net2.get_nic().unwrap();
        { let mut w = wan.lock().await; w.add_net(Arc::clone(&nic1)).await.unwrap(); w.add_net(Arc::clone(&nic2)).await.unwrap(); }
        { nic1.lock().await.set_router(Arc::clone(&wan)).await.unwrap(); }
        { nic2.lock().await.set_router(Arc::clone(&wan)).await.unwrap(); }
        // CRITICAL: start the virtual network
        { wan.lock().await.start().await.unwrap(); }
        let mut se1 = SettingEngine::default(); se1.set_vnet(Some(net1.clone())); se1.set_ice_timeouts(Some(Duration::from_secs(5)), Some(Duration::from_secs(5)), Some(Duration::from_millis(200)));
        let mut se2 = SettingEngine::default(); se2.set_vnet(Some(net2.clone())); se2.set_ice_timeouts(Some(Duration::from_secs(5)), Some(Duration::from_secs(5)), Some(Duration::from_millis(200)));
        (se1, se2, wan)
    });

    let mut pc1 = NativePeerConnection::with_setting_engine(Some(se1)).expect("pc1");
    let mut pc2 = NativePeerConnection::with_setting_engine(Some(se2)).expect("pc2");

    let (tx1, rx1) = std::sync::mpsc::channel::<IceCandidate>();
    let (tx2, rx2) = std::sync::mpsc::channel::<IceCandidate>();
    pc1.set_on_ice_candidate(Box::new(move |c| { let _ = tx2.send(c); }));
    pc2.set_on_ice_candidate(Box::new(move |c| { let _ = tx1.send(c); }));

    let pc1_state = Arc::new(Mutex::new(IceConnectionState::New));
    let pc2_state = Arc::new(Mutex::new(IceConnectionState::New));
    let s1 = pc1_state.clone(); let s2 = pc2_state.clone();
    pc1.set_on_ice_connection_state_change(Box::new(move |s| { *s1.lock().unwrap() = s; }));
    pc2.set_on_ice_connection_state_change(Box::new(move |s| { *s2.lock().unwrap() = s; }));

    let mut g = VideoFrameGenerator::new(W, H, FPS); g.start();
    let _track = pc1.create_video_track(Box::new(g)).expect("create_video_track");

    let received = Arc::new(Mutex::new(0u32)); let rf = received.clone();
    let on_track_triggered = Arc::new(Mutex::new(false));
    let ot = on_track_triggered.clone();
    pc2.set_on_track(Box::new(move |track: Box<dyn VideoTrack>| {
        *ot.lock().unwrap() = true;
        let r = rf.clone();
        struct Sink { count: Arc<Mutex<u32>> }
        impl VideoSink<gkit_media::video::frame::BoxVideoFrame> for Sink {
            fn on_frame(&self, _f: &gkit_media::video::frame::BoxVideoFrame) { *self.count.lock().unwrap() += 1; }
        }
        track.add_sink(Box::new(Sink { count: r }));
    }));

    // Non-trickle ICE: gather candidates into SDP, exchange via set_remote_description
    let offer = pc1.create_offer().expect("offer");
    pc1.set_local_description(&offer).expect("set local1");
    pc1.gather_complete().ok();
    // Use gathered LOCAL description (includes candidates in SDP)
    let offer_with_cands = pc1.local_description().expect("local desc1");
    eprintln!("[test] offer SDP size={}", offer_with_cands.sdp.len());
    pc2.set_remote_description(&offer_with_cands).expect("set remote2");

    let answer = pc2.create_answer().expect("answer");
    pc2.set_local_description(&answer).expect("set local2");
    pc2.gather_complete().ok();
    let answer_with_cands = pc2.local_description().expect("local desc2");
    eprintln!("[test] answer SDP size={}", answer_with_cands.sdp.len());
    pc1.set_remote_description(&answer_with_cands).expect("set remote1");

    // Check if any tracks were received
    let _ = &answer;
    let _ = &answer_with_cands;

    eprintln!("[test] post-exchange: pc1_ice={:?} pc2_ice={:?} pc2_signaling={:?} pc2_connection={:?}", pc1.ice_connection_state(), pc2.ice_connection_state(), pc2.signaling_state(), pc2.connection_state());

    // Also exchange explicit candidates for trickle fallback
    for c in rx2.try_iter() { pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
    for c in rx1.try_iter() { pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }

    eprintln!("[test] offer SDP:\n{}", &offer_with_cands.sdp[..offer_with_cands.sdp.len().min(2000)]);
    eprintln!("[test] answer SDP:\n{}", &answer_with_cands.sdp[..answer_with_cands.sdp.len().min(2000)]);

    let start = Instant::now();
    loop {
        let frames = *received.lock().unwrap();
        if frames >= 5 { break; }
        if start.elapsed() > Duration::from_secs(30) {
            panic!("timeout: pc1={:?} pc2={:?} track={:?} frames={}", *pc1_state.lock().unwrap(), *pc2_state.lock().unwrap(), *on_track_triggered.lock().unwrap(), frames);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    eprintln!("[test] vnet P2P frames received: {}", received.lock().unwrap());
    pc1.close().ok(); pc2.close().ok();
}
