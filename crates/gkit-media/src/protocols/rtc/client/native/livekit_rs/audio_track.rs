use libwebrtc::audio_track::RtcAudioTrack as LkRtcAudioTrack;

/// Thin adapter wrapping a libwebrtc `RtcAudioTrack`.
///
/// Provides `id()`, `kind()`, and `enabled()` accessors mirroring the `VideoTrack` trait
/// pattern. A formal `AudioTrack` trait may be added to gkit-core later.
pub struct LkAudioTrack {
    pub(crate) inner: LkRtcAudioTrack,
    id: String,
}

impl LkAudioTrack {
    pub fn new(inner: LkRtcAudioTrack) -> Self {
        let id = inner.id();
        Self { inner, id }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> &str {
        "audio"
    }

    pub fn enabled(&self) -> bool {
        self.inner.enabled()
    }

    pub fn set_enabled(&self, enabled: bool) -> bool {
        self.inner.set_enabled(enabled)
    }
}
