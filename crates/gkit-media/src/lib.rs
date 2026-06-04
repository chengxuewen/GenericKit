use gkit_core::core_hello;

pub fn media_hello() {
    core_hello();
    println!("media_hello!");
}

pub mod capture;
#[cfg(all(feature = "plugin", not(target_arch = "wasm32")))]
pub mod plugin;
pub mod protocols;
pub mod video;
#[path = "trait/mod.rs"]
mod trait_mod;
pub use trait_mod::{video_sink_stabby, webrtc_stabby};

#[cfg(all(feature = "plugin", not(target_arch = "wasm32")))]
pub fn make_peer_connection() -> Box<dyn protocols::rtc::peer::PeerConnection> {
    register_test_backend();
    use protocols::rtc::peer::RtcEngine;
    RtcEngine::create_default()
        .expect("no RTC backend registered")
        .create_peer_connection()
        .expect("failed to create PeerConnection")
}

#[cfg(all(feature = "plugin", not(target_arch = "wasm32")))]
pub fn make_peer_connection_with_backend(name: &str) -> protocols::rtc::peer::MediaResult<Box<dyn protocols::rtc::peer::PeerConnection>> {
    use protocols::rtc::peer::RtcEngine;
    RtcEngine::create(name)?.create_peer_connection()
}

#[cfg(all(feature = "plugin", not(target_arch = "wasm32")))]
pub fn register_test_backend() {
    use protocols::rtc::peer::RtcEngine;
    use protocols::rtc::peer::{ConnectionState, DataChannel, DataChannelState, GatheringState, IceConnectionState, MediaError, MediaResult, PeerConnection, PeerConnectionFactory, RtcConfiguration, SessionDescription, SignalingState};
    use std::sync::OnceLock;
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        struct MockPC(std::sync::atomic::AtomicBool);
        impl PeerConnection for MockPC {
            fn create_offer(&self) -> MediaResult<SessionDescription> { if self.0.load(std::sync::atomic::Ordering::Relaxed) { Err(MediaError::new("closed")) } else { Ok(SessionDescription{sdp_type:"offer".into(),sdp:String::new()}) }}
            fn create_answer(&self) -> MediaResult<SessionDescription> { if self.0.load(std::sync::atomic::Ordering::Relaxed) { Err(MediaError::new("closed")) } else { Ok(SessionDescription{sdp_type:"answer".into(),sdp:String::new()}) }}
            fn set_local_description(&mut self, _:&SessionDescription) -> MediaResult<()> { if self.0.load(std::sync::atomic::Ordering::Relaxed) { Err(MediaError::new("closed")) } else { Ok(()) } }
            fn set_remote_description(&mut self, _:&SessionDescription) -> MediaResult<()> { if self.0.load(std::sync::atomic::Ordering::Relaxed) { Err(MediaError::new("closed")) } else { Ok(()) } }
            fn add_ice_candidate(&mut self, _:&str, _:&str) -> MediaResult<()> { if self.0.load(std::sync::atomic::Ordering::Relaxed) { Err(MediaError::new("closed")) } else { Ok(()) } }
            fn create_data_channel(&self, l:&str) -> MediaResult<Box<dyn DataChannel>> {
                if self.0.load(std::sync::atomic::Ordering::Relaxed) { return Err(MediaError::new("closed")); }
                struct DC{label:String,closed:std::sync::atomic::AtomicBool} impl DataChannel for DC { fn label(&self)->&str{&self.label} fn ready_state(&self)->DataChannelState{if self.closed.load(std::sync::atomic::Ordering::Relaxed){DataChannelState::Closed}else{DataChannelState::Open}} fn send_text(&self,_:&str)->MediaResult<()>{if self.closed.load(std::sync::atomic::Ordering::Relaxed){Err(MediaError::new("closed"))}else{Ok(())}} fn send_bytes(&self,_:&[u8])->MediaResult<()>{if self.closed.load(std::sync::atomic::Ordering::Relaxed){Err(MediaError::new("closed"))}else{Ok(())}} fn close(&mut self)->MediaResult<()>{self.closed.store(true,std::sync::atomic::Ordering::Relaxed);Ok(())} }
                Ok(Box::new(DC{label:l.into(),closed:std::sync::atomic::AtomicBool::new(false)}))
            }
            fn ice_connection_state(&self) -> IceConnectionState { if self.0.load(std::sync::atomic::Ordering::Relaxed) { IceConnectionState::Closed } else { IceConnectionState::New } }
            fn connection_state(&self) -> ConnectionState { ConnectionState::New }
            fn gathering_state(&self) -> GatheringState { GatheringState::New }
            fn signaling_state(&self) -> SignalingState { SignalingState::Stable }
            fn local_description(&self) -> MediaResult<SessionDescription> { Err(MediaError::new("no local desc")) }
            fn remote_description(&self) -> MediaResult<SessionDescription> { Err(MediaError::new("no remote desc")) }
            fn close(&mut self) -> MediaResult<()> { self.0.store(true, std::sync::atomic::Ordering::Relaxed); Ok(()) }
        }
        struct MockF; impl PeerConnectionFactory for MockF {
            fn backend_name(&self) -> &'static str { "webrtc-rs" }
            fn create_peer_connection(&self) -> MediaResult<Box<dyn PeerConnection>> { Ok(Box::new(MockPC(std::sync::atomic::AtomicBool::new(false)))) }
            fn create_peer_connection_with_config(&self, _:&RtcConfiguration) -> MediaResult<Box<dyn PeerConnection>> { self.create_peer_connection() }
        }
        RtcEngine::register("webrtc-rs", || Box::new(MockF));
    });
}
