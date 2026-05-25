use crate::protocols::rtc::client::core::PeerConnectionFactory;

mod audio_track;
mod data_channel;
mod factory;
mod ice;
mod peer_connection;
mod session_description;
mod video_frame;
mod video_track;
pub use factory::LiveKitRsFactory;
pub use session_description::lk_sdp_from_core;

crate::gkit_register_rtc_backend!("google", LiveKitRsFactory);

pub fn create_factory() -> Box<dyn PeerConnectionFactory> {
    Box::new(LiveKitRsFactory::new())
}
