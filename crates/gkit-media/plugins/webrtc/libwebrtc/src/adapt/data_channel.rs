use libwebrtc::data_channel::{DataChannel as LkDataChannel, DataChannelState as LkDataChannelState};

use gkit_media::protocols::rtc::client::core::{DataChannel, DataChannelState, MediaError, MediaResult};


/// Thin adapter wrapping a libwebrtc `DataChannel` to implement gkit's `DataChannel` trait.
pub(super) struct LkDataChannelAdapter {
    inner: LkDataChannel,
    label: String,
}

impl LkDataChannelAdapter {
    pub fn new(inner: LkDataChannel) -> Self {
        let label = inner.label();
        Self { inner, label }
    }
}

impl DataChannel for LkDataChannelAdapter {
    fn label(&self) -> &str {
        &self.label
    }

    fn ready_state(&self) -> DataChannelState {
        crate::adapt::convert::lk_dc_state(self.inner.state())
    }

    fn send_text(&self, data: &str) -> MediaResult<()> {
        self.inner
            .send(data.as_bytes(), false)
            .map_err(|e| MediaError::new(format!("data channel send: {e}")))
    }

    fn send_bytes(&self, data: &[u8]) -> MediaResult<()> {
        self.inner
            .send(data, true)
            .map_err(|e| MediaError::new(format!("data channel send: {e}")))
    }

    fn stream_id(&self) -> MediaResult<u32> {
        Ok(self.inner.id() as u32)
    }

    fn close(&mut self) -> MediaResult<()> {
        self.inner.close();
        Ok(())
    }
}
