use crate::video::frame_stabby::StableVideoFrame;

#[stabby::stabby(checked)]
pub trait IStableVideoSink {
    extern "C" fn on_frame_owned(&self, frame: stabby::boxed::Box<StableVideoFrame>);
    extern "C" fn on_frame<'a>(&'a self, frame: &'a StableVideoFrame);
    extern "C" fn on_discarded_frame(&self, timestamp_us: i64);
}
