/// Video frame rotation (W3C compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoRotation {
    Rotation0 = 0,
    Rotation90 = 90,
    Rotation180 = 180,
    Rotation270 = 270,
}

impl Default for VideoRotation {
    fn default() -> Self {
        Self::Rotation0
    }
}

/// Frame metadata for packet trailer features.
#[derive(Debug, Clone, Default)]
pub struct FrameMetadata {
    pub user_timestamp: Option<u64>,
    pub frame_id: Option<u32>,
}

/// A video frame containing a buffer of type T.
#[derive(Debug, Clone)]
pub struct VideoFrame<T> {
    pub rotation: VideoRotation,
    pub timestamp_us: i64,
    pub metadata: Option<FrameMetadata>,
    pub buffer: T,
}

impl<T> VideoFrame<T> {
    pub fn new(buffer: T) -> Self {
        Self {
            rotation: VideoRotation::default(),
            timestamp_us: 0,
            metadata: None,
            buffer,
        }
    }

    pub fn with_rotation(mut self, rotation: VideoRotation) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_timestamp(mut self, ts: i64) -> Self {
        self.timestamp_us = ts;
        self
    }

    pub fn with_metadata(mut self, meta: FrameMetadata) -> Self {
        self.metadata = Some(meta);
        self
    }
}

impl VideoFrame<Box<dyn super::buffer::VideoBuffer>> {
    pub fn width(&self) -> u32 {
        self.buffer.width()
    }
    pub fn height(&self) -> u32 {
        self.buffer.height()
    }
}

/// Type alias for heap-allocated video frame.
pub type BoxVideoFrame = VideoFrame<Box<dyn super::buffer::VideoBuffer>>;
