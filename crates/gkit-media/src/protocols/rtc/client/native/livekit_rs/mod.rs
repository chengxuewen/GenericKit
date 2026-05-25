use crate::protocols::rtc::client::core::PeerConnectionFactory;

mod factory;
mod peer_connection;
pub use factory::LiveKitRsFactory;

crate::gkit_register_rtc_backend!("google", LiveKitRsFactory);

pub fn create_factory() -> Box<dyn PeerConnectionFactory> {
    Box::new(LiveKitRsFactory::new())
}
