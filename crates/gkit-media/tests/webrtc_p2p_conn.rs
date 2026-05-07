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
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (se1, se2, _n1, _n2, _wan) = rt.block_on(async {
        let wan = Arc::new(tokio::sync::Mutex::new(Router::new(RouterConfig { cidr: "1.2.3.0/24".to_string(), ..Default::default() }).unwrap()));
        let net1 = Arc::new(Net::new(Some(NetConfig { static_ips: vec!["1.2.3.4".to_string()], ..Default::default() })));
        let net2 = Arc::new(Net::new(Some(NetConfig { static_ips: vec!["1.2.3.5".to_string()], ..Default::default() })));
        { let mut w = wan.lock().await; w.add_net(net1.get_nic().unwrap()).await.unwrap(); w.add_net(net2.get_nic().unwrap()).await.unwrap(); }
        { net1.get_nic().unwrap().lock().await.set_router(wan.clone()).await.unwrap(); }
        { net2.get_nic().unwrap().lock().await.set_router(wan.clone()).await.unwrap(); }
        let mut se1 = SettingEngine::default(); se1.set_vnet(Some(net1.clone())); se1.set_ice_timeouts(Some(Duration::from_secs(5)), Some(Duration::from_secs(5)), Some(Duration::from_millis(200)));
        let mut se2 = SettingEngine::default(); se2.set_vnet(Some(net2.clone())); se2.set_ice_timeouts(Some(Duration::from_secs(5)), Some(Duration::from_secs(5)), Some(Duration::from_millis(200)));
        (se1, se2, net1, net2, wan)
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
    pc1.create_video_track(Box::new(g)).expect("create_video_track");

    let received = Arc::new(Mutex::new(0u32)); let rf = received.clone();
    pc2.set_on_track(Box::new(move |track: Box<dyn VideoTrack>| {
        let r = rf.clone();
        struct Sink { count: Arc<Mutex<u32>> }
        impl VideoSink<gkit_media::video::frame::BoxVideoFrame> for Sink {
            fn on_frame(&self, _f: &gkit_media::video::frame::BoxVideoFrame) { *self.count.lock().unwrap() += 1; }
        }
        track.add_sink(Box::new(Sink { count: r }));
    }));

    let offer = pc1.create_offer().expect("offer");
    eprintln!("[test] offer SDP m-line present: {}", offer.sdp.contains("m=video"));
    pc1.set_local_description(&offer).expect("set local1");
    pc2.set_remote_description(&offer).expect("set remote2");
    let answer = pc2.create_answer().expect("answer");
    pc2.set_local_description(&answer).expect("set local2");
    pc1.set_remote_description(&answer).expect("set remote1");

    // Wait for gather + exchange continuously
    pc1.gather_complete().ok(); pc2.gather_complete().ok();
    let mut total = 0u32;
    for c in rx2.try_iter() { total += 1; pc2.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
    for c in rx1.try_iter() { total += 1; pc1.add_ice_candidate(&c.candidate, c.sdp_mid.as_deref().unwrap_or("")).ok(); }
    eprintln!("[test] candidates exchanged: {}", total);

    let start = Instant::now();
    loop {
        let frames = *received.lock().unwrap();
        if frames >= 5 { break; }
        if start.elapsed() > Duration::from_secs(30) {
            panic!("timeout: pc1={:?} pc2={:?} frames={}", *pc1_state.lock().unwrap(), *pc2_state.lock().unwrap(), frames);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    eprintln!("[test] vnet P2P frames received: {}", received.lock().unwrap());
    pc1.close().ok(); pc2.close().ok();
}
