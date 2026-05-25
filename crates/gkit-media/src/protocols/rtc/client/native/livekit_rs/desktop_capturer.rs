//! DesktopCapturer adapter wrapping `libwebrtc::desktop_capturer`.
//!
//! Provides screen / window capture via LiveKit's libwebrtc fork.
//! Only available on desktop platforms (macOS, Windows, Linux).

use libwebrtc::desktop_capturer as lk_dc;

// ---------------------------------------------------------------------------
// Re-export libwebrtc types so callers don't need to depend on libwebrtc
// directly for basic usage.
// ---------------------------------------------------------------------------

/// Source type to capture (screen, window, or generic).
pub use lk_dc::DesktopCaptureSourceType;

/// Error returned when capture fails (temporary or permanent).
pub use lk_dc::CaptureError;

/// A single captured desktop frame.
pub use lk_dc::DesktopFrame;

/// A source (display or window) that can be captured.
pub use lk_dc::CaptureSource;

// ---------------------------------------------------------------------------
// DesktopCapturerOptions
// ---------------------------------------------------------------------------

/// Configuration options for the desktop capturer.
pub struct LkDesktopCapturerOptions {
    inner: lk_dc::DesktopCapturerOptions,
}

impl LkDesktopCapturerOptions {
    pub fn new(source_type: DesktopCaptureSourceType) -> Self {
        Self {
            inner: lk_dc::DesktopCapturerOptions::new(source_type),
        }
    }

    pub fn set_include_cursor(&mut self, include: bool) {
        self.inner.set_include_cursor(include);
    }

    #[cfg(target_os = "macos")]
    pub fn set_sck_system_picker(&mut self, allow: bool) {
        self.inner.set_sck_system_picker(allow);
    }
}

// ---------------------------------------------------------------------------
// DesktopCapturer
// ---------------------------------------------------------------------------

/// Captures screen or window content.
pub struct LkDesktopCapturer {
    inner: lk_dc::DesktopCapturer,
}

impl LkDesktopCapturer {
    /// Creates a new capturer. Returns `None` if creation failed (e.g.
    /// missing permissions or unsupported platform).
    pub fn new(options: LkDesktopCapturerOptions) -> Option<Self> {
        lk_dc::DesktopCapturer::new(options.inner).map(|inner| Self { inner })
    }

    /// Initialises a capture session from the given source (or system picker).
    /// The `callback` is invoked for each captured frame.
    pub fn start_capture<T>(&mut self, source: Option<CaptureSource>, callback: T)
    where
        T: FnMut(Result<DesktopFrame, CaptureError>) + Send + 'static,
    {
        self.inner.start_capture(source, callback);
    }

    /// Triggers capture of a single frame.
    /// Must call [`start_capture`](Self::start_capture) first.
    pub fn capture_frame(&mut self) {
        self.inner.capture_frame();
    }

    /// Lists available capture sources (displays or windows).
    pub fn get_source_list(&self) -> Vec<CaptureSource> {
        self.inner.get_source_list()
    }
}
