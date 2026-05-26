//! Stabby-compatible video frame types for cross-dylib ABI stability.
//!
//! These types parallel the non-stabby types in [`super::frame`] and [`super::buffer`],
//! but use `#[stabby::stabby]` for ABI-safe layout and `stabby::sync::ArcSlice`
//! for pixel data ownership across shared library boundaries.
//!
//! # Relationship to existing types
//!
//! | Non-stabby | Stabby equivalent |
//! |------------|-------------------|
//! | `VideoRotation` (enum) | `u8` (0/90/180/270) in `VideoFrameMeta.rotation` |
//! | `VideoFrame<T>` (generic) | `StableVideoFrame` (non-generic, enum-based buffer) |
//! | `VideoBuffer` (trait) | `BufferData` (enum) |
//! | `I420Buffer` (struct) | `I420Planes` (struct with `ArcSlice<u8>`) |
//!
//! # Safety
//!
//! `stabby::sync::ArcSlice<u8>` uses a vtable-stored destructor, ensuring pixel data
//! allocated in a cdylib can be safely dropped by the host even if the cdylib is unloaded
//! (though our architecture keeps plugins loaded for the process lifetime).

use stabby::sync::ArcSlice;

// ============================================================================
// VideoFrameMeta — frame header metadata (ABI-stable)
// ============================================================================

/// Frame-level metadata in an ABI-stable layout.
///
/// Uses `u8` for rotation (0, 90, 180, 270) instead of a Rust enum
/// to guarantee layout stability across compiler versions.
#[stabby::stabby]
#[derive(Debug, Clone)]
pub struct VideoFrameMeta {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Rotation: 0, 90, 180, or 270 degrees (W3C-compatible).
    pub rotation: u8,
    /// Presentation timestamp in microseconds.
    pub timestamp_us: i64,
}

impl VideoFrameMeta {
    /// Create metadata with the given dimensions, zero rotation, and zero timestamp.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rotation: 0,
            timestamp_us: 0,
        }
    }

    /// Set rotation (0, 90, 180, 270).
    #[must_use]
    pub fn with_rotation(mut self, rotation: u8) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set presentation timestamp.
    #[must_use]
    pub fn with_timestamp(mut self, ts: i64) -> Self {
        self.timestamp_us = ts;
        self
    }
}

// ============================================================================
// I420Planes — planar 4:2:0 YUV pixel data
// ============================================================================

/// Planar 4:2:0 YUV buffer with ABI-stable `ArcSlice<u8>` ownership.
///
/// Chroma planes are half the resolution of the luma plane.
#[stabby::stabby]
#[derive(Debug, Clone)]
pub struct I420Planes {
    /// Luma plane data (width × height = `stride_y * height` bytes).
    pub data_y: ArcSlice<u8>,
    /// U chroma plane data (chroma_width × chroma_height = `stride_u * (height+1)/2` bytes).
    pub data_u: ArcSlice<u8>,
    /// V chroma plane data.
    pub data_v: ArcSlice<u8>,
    /// Luma plane stride in bytes.
    pub stride_y: u32,
    /// Chroma U plane stride in bytes.
    pub stride_u: u32,
    /// Chroma V plane stride in bytes.
    pub stride_v: u32,
}

impl I420Planes {
    /// Create I420 planes with pre-allocated pixel data.
    #[must_use]
    pub fn new(
        width: u32,
        _height: u32,
        data_y: Vec<u8>,
        data_u: Vec<u8>,
        data_v: Vec<u8>,
    ) -> Self {
        let stride_y = width;
        let stride_u = (width + 1) / 2;
        let stride_v = (width + 1) / 2;
        Self {
            data_y: data_y.into_iter().collect(),
            data_u: data_u.into_iter().collect(),
            data_v: data_v.into_iter().collect(),
            stride_y,
            stride_u,
            stride_v,
        }
    }

    /// Create empty I420 planes of the given dimensions (all zeros for Y, 128 for UV).
    #[must_use]
    pub fn zeroed(width: u32, height: u32) -> Self {
        let stride_y = width;
        let stride_u = (width + 1) / 2;
        let stride_v = (width + 1) / 2;
        let size_y = (stride_y * height) as usize;
        let size_u = (stride_u * ((height + 1) / 2)) as usize;
        let size_v = (stride_v * ((height + 1) / 2)) as usize;
        Self {
            data_y: ArcSlice::from(&vec![0u8; size_y][..]),
            data_u: ArcSlice::from(&vec![128u8; size_u][..]),
            data_v: ArcSlice::from(&vec![128u8; size_v][..]),
            stride_y,
            stride_u,
            stride_v,
        }
    }
}

// ============================================================================
// NV12Planes — semi-planar 4:2:0 Y+UV pixel data
// ============================================================================

/// Semi-planar NV12 buffer with ABI-stable `ArcSlice<u8>` ownership.
///
/// UV plane is interleaved: UVUVUV...
/// Chroma resolution is half the luma resolution in both dimensions.
#[stabby::stabby]
#[derive(Debug, Clone)]
pub struct NV12Planes {
    /// Luma plane data (width × height = `stride_y * height` bytes).
    pub data_y: ArcSlice<u8>,
    /// Interleaved UV chroma plane data (`stride_uv * (height+1)/2` bytes).
    pub data_uv: ArcSlice<u8>,
    /// Luma plane stride in bytes.
    pub stride_y: u32,
    /// Interleaved UV plane stride in bytes.
    pub stride_uv: u32,
}

impl NV12Planes {
    pub fn new(width: u32, _height: u32, data_y: Vec<u8>, data_uv: Vec<u8>) -> Self {
        let stride_y = width;
        let stride_uv = width;
        Self {
            data_y: data_y.into_iter().collect(),
            data_uv: data_uv.into_iter().collect(),
            stride_y,
            stride_uv,
        }
    }
    pub fn zeroed(width: u32, height: u32) -> Self {
        let stride_y = width;
        let stride_uv = width;
        let size_y = (stride_y * height) as usize;
        let size_uv = (stride_uv * ((height + 1) / 2)) as usize;
        Self {
            data_y: ArcSlice::from(&vec![0u8; size_y][..]),
            data_uv: ArcSlice::from(&vec![128u8; size_uv][..]),
            stride_y,
            stride_uv,
        }
    }
}

// ============================================================================
// BufferData — discriminated buffer type
// ============================================================================

/// ABI-stable enum carrying one of several buffer formats.
///
/// New formats can be added as variants without breaking the
/// existing ABI (stabby assigns stable discriminants).
#[stabby::stabby]
#[derive(Debug, Clone)]
pub enum BufferData {
    /// Planar 4:2:0 YUV.
    I420(I420Planes),
    /// Semi-planar 4:2:0 Y+UV (NV12).
    NV12(NV12Planes),
}

impl BufferData {
    #[must_use]
    pub fn is_i420(&self) -> bool {
        format!("{self:?}").contains("I420")
    }

    #[must_use]
    pub fn is_nv12(&self) -> bool {
        format!("{self:?}").contains("NV12")
    }
}

// ============================================================================
// StableVideoFrame — ABI-stable video frame
// ============================================================================

/// A fully ABI-stable video frame that can cross cdylib boundaries.
///
/// Unlike the generic [`super::frame::VideoFrame<T>`], this type uses a
/// concrete [`BufferData`] enum instead of a type parameter, making it
/// suitable for dynamic dispatch across plugin boundaries via `stabby`.
#[stabby::stabby]
#[derive(Debug, Clone)]
pub struct StableVideoFrame {
    /// Frame-level metadata (dimensions, rotation, timestamp).
    pub meta: VideoFrameMeta,
    /// Pixel data in one of several supported formats.
    pub buffer: BufferData,
}

impl StableVideoFrame {
    /// Create a new frame from metadata and buffer data.
    #[must_use]
    pub fn new(meta: VideoFrameMeta, buffer: BufferData) -> Self {
        Self { meta, buffer }
    }

    /// Create a new frame with zeroed I420 data at the given dimensions.
    #[must_use]
    pub fn new_i420(width: u32, height: u32) -> Self {
        Self {
            meta: VideoFrameMeta::new(width, height),
            buffer: BufferData::I420(I420Planes::zeroed(width, height)),
        }
    }
}

// ============================================================================
// Tests (inline)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_new_defaults() {
        let m = VideoFrameMeta::new(640, 480);
        assert_eq!(m.width, 640);
        assert_eq!(m.height, 480);
        assert_eq!(m.rotation, 0);
        assert_eq!(m.timestamp_us, 0);
    }

    #[test]
    fn meta_builder() {
        let m = VideoFrameMeta::new(1920, 1080)
            .with_rotation(90)
            .with_timestamp(42_000);
        assert_eq!(m.rotation, 90);
        assert_eq!(m.timestamp_us, 42_000);
    }

    #[test]
    fn i420_zeroed_dimensions() {
        let p = I420Planes::zeroed(640, 480);
        assert_eq!(p.stride_y, 640);
        assert_eq!(p.stride_u, 320);
        assert_eq!(p.stride_v, 320);
        // data_y: 640 * 480 = 307200 bytes
        assert_eq!(p.data_y.len(), 640 * 480);
        // data_u/v: 320 * 240 = 76800 bytes each
        assert_eq!(p.data_u.len(), 320 * 240);
        assert_eq!(p.data_v.len(), 320 * 240);
    }

    #[test]
    fn nv12_zeroed_dimensions() {
        let p = NV12Planes::zeroed(1920, 1080);
        assert_eq!(p.stride_y, 1920);
        assert_eq!(p.stride_uv, 1920);
        // data_y: 1920 * 1080
        assert_eq!(p.data_y.len(), 1920 * 1080);
        // data_uv: 1920 * 540 = 1036800
        assert_eq!(p.data_uv.len(), 1920 * 540);
    }

    #[test]
    fn buffer_enum_i420_variant() {
        let planes = I420Planes::zeroed(320, 240);
        let buf = BufferData::I420(planes);
        assert!(format!("{buf:?}").contains("I420"));
    }

    #[test]
    fn buffer_enum_nv12_variant() {
        let planes = NV12Planes::zeroed(320, 240);
        let buf = BufferData::NV12(planes);
        assert!(format!("{buf:?}").contains("NV12"));
    }
}
