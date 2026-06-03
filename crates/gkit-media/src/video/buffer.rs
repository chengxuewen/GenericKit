use std::fmt;

/// Supported video buffer types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoBufferType {
    Native,
    I420,
    I420A,
    I422,
    I444,
    I010,
    NV12,
}

/// RGBA format variants for conversion output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFormatType {
    ARGB,
    BGRA,
    ABGR,
    RGBA,
}

/// Trait for video frame buffers.
pub trait VideoBuffer: fmt::Debug + Send + Sync {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn buffer_type(&self) -> VideoBufferType;

    fn as_i420(&self) -> Option<&I420Buffer> { None }
    fn as_i422(&self) -> Option<&I422Buffer> { None }
    fn as_i444(&self) -> Option<&I444Buffer> { None }
    fn as_nv12(&self) -> Option<&NV12Buffer> { None }
    fn as_i010(&self) -> Option<&I010Buffer> { None }

    /// Scale this buffer to a new resolution.
    fn scale(&self, scaled_width: u32, scaled_height: u32) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError>;

    /// Convert to I420 format.
    fn to_i420(&self) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError>;
}

// ============================================================================
// I420Buffer — planar 4:2:0 YUV, 8-bit
// ============================================================================

#[derive(Debug, Clone)]
pub struct I420Buffer {
    pub width: u32,
    pub height: u32,
    pub stride_y: u32,
    pub stride_u: u32,
    pub stride_v: u32,
    pub data_y: Vec<u8>,
    pub data_u: Vec<u8>,
    pub data_v: Vec<u8>,
}

impl I420Buffer {
    pub fn new(width: u32, height: u32) -> Self {
        let stride_y = width;
        let stride_u = (width + 1) / 2;
        let stride_v = (width + 1) / 2;
        let size_y = (stride_y * height) as usize;
        let size_u = (stride_u * ((height + 1) / 2)) as usize;
        let size_v = (stride_v * ((height + 1) / 2)) as usize;
        Self {
            width, height, stride_y, stride_u, stride_v,
            data_y: vec![0u8; size_y],
            data_u: vec![128u8; size_u],
            data_v: vec![128u8; size_v],
        }
    }

    pub fn chroma_width(&self) -> u32 { (self.width + 1) / 2 }
    pub fn chroma_height(&self) -> u32 { (self.height + 1) / 2 }
}

impl VideoBuffer for I420Buffer {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn buffer_type(&self) -> VideoBufferType { VideoBufferType::I420 }
    fn as_i420(&self) -> Option<&I420Buffer> { Some(self) }

    fn scale(&self, scaled_width: u32, scaled_height: u32) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        let mut out = I420Buffer::new(scaled_width, scaled_height);
        let x_ratio = self.width as f64 / scaled_width as f64;
        let y_ratio = self.height as f64 / scaled_height as f64;

        // Y plane (full resolution)
        for y in 0..scaled_height {
            for x in 0..scaled_width {
                let sx = (x as f64 * x_ratio) as u32;
                let sy = (y as f64 * y_ratio) as u32;
                let src_idx = (sy * self.stride_y + sx) as usize;
                let dst_idx = (y * out.stride_y + x) as usize;
                out.data_y[dst_idx] = self.data_y[src_idx];
            }
        }

        // U/V planes (chroma: half resolution, same ratios)
        let cw = out.chroma_width();
        let ch = out.chroma_height();
        for y in 0..ch {
            for x in 0..cw {
                let sx = (x as f64 * x_ratio) as u32;
                let sy = (y as f64 * y_ratio) as u32;
                let src_idx = (sy * self.stride_u + sx) as usize;
                let dst_idx = (y * out.stride_u + x) as usize;
                out.data_u[dst_idx] = self.data_u[src_idx];
                out.data_v[dst_idx] = self.data_v[src_idx];
            }
        }

        Ok(out)
    }

    fn to_i420(&self) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        Ok(self.clone())
    }
}

// ============================================================================
// I422Buffer — planar 4:2:2 YUV, 8-bit
// ============================================================================

#[derive(Debug, Clone)]
pub struct I422Buffer {
    pub width: u32,
    pub height: u32,
    pub stride_y: u32,
    pub stride_u: u32,
    pub stride_v: u32,
    pub data_y: Vec<u8>,
    pub data_u: Vec<u8>,
    pub data_v: Vec<u8>,
}

impl I422Buffer {
    pub fn new(width: u32, height: u32) -> Self {
        let stride_y = width;
        let stride_u = (width + 1) / 2;
        let stride_v = (width + 1) / 2;
        Self {
            width, height, stride_y, stride_u, stride_v,
            data_y: vec![0u8; (stride_y * height) as usize],
            data_u: vec![128u8; (stride_u * height) as usize],
            data_v: vec![128u8; (stride_v * height) as usize],
        }
    }
}

impl VideoBuffer for I422Buffer {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn buffer_type(&self) -> VideoBufferType { VideoBufferType::I422 }
    fn as_i422(&self) -> Option<&I422Buffer> { Some(self) }

    fn scale(&self, w: u32, h: u32) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        self.to_i420()?.scale(w, h)
    }

    fn to_i420(&self) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        // Stub: vertical chroma subsample (4:2:2 → 4:2:0)
        let mut out = I420Buffer::new(self.width, self.height);
        out.data_y.copy_from_slice(&self.data_y);
        // Average every 2 chroma rows
        for y in 0..out.chroma_height() {
            let src_row = (y * 2) as usize;
            let src_off = src_row * self.stride_u as usize;
            let dst_off = (y * out.stride_u) as usize;
            for x in 0..out.stride_u as usize {
                let v0 = self.data_u[src_off + x];
                let v1 = self.data_u.get(src_off + self.stride_u as usize + x).copied().unwrap_or(v0);
                out.data_u[dst_off + x] = ((v0 as u16 + v1 as u16) / 2) as u8;
                let w0 = self.data_v[src_off + x];
                let w1 = self.data_v.get(src_off + self.stride_v as usize + x).copied().unwrap_or(w0);
                out.data_v[dst_off + x] = ((w0 as u16 + w1 as u16) / 2) as u8;
            }
        }
        Ok(out)
    }
}

// ============================================================================
// I444Buffer — planar 4:4:4 YUV, 8-bit
// ============================================================================

#[derive(Debug, Clone)]
pub struct I444Buffer {
    pub width: u32,
    pub height: u32,
    pub stride_y: u32,
    pub stride_u: u32,
    pub stride_v: u32,
    pub data_y: Vec<u8>,
    pub data_u: Vec<u8>,
    pub data_v: Vec<u8>,
}

impl I444Buffer {
    pub fn new(width: u32, height: u32) -> Self {
        let stride = width;
        Self {
            width, height,
            stride_y: stride, stride_u: stride, stride_v: stride,
            data_y: vec![0u8; (stride * height) as usize],
            data_u: vec![128u8; (stride * height) as usize],
            data_v: vec![128u8; (stride * height) as usize],
        }
    }
}

impl VideoBuffer for I444Buffer {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn buffer_type(&self) -> VideoBufferType { VideoBufferType::I444 }
    fn as_i444(&self) -> Option<&I444Buffer> { Some(self) }

    fn scale(&self, w: u32, h: u32) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        self.to_i420()?.scale(w, h)
    }

    fn to_i420(&self) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        // Stub: 4:4:4 → 4:2:0 (subsample chroma both directions)
        let mut out = I420Buffer::new(self.width, self.height);
        out.data_y.copy_from_slice(&self.data_y);
        for y in 0..out.chroma_height() {
            let sy = (y * 2) as usize;
            for x in 0..out.chroma_width() {
                let sx = (x * 2) as usize;
                let s00 = self.data_u[sy * self.stride_u as usize + sx];
                let s10 = self.data_u[sy * self.stride_u as usize + sx + 1];
                let s01 = self.data_u.get((sy + 1) * self.stride_u as usize + sx).copied().unwrap_or(s00);
                let s11 = self.data_u.get((sy + 1) * self.stride_u as usize + sx + 1).copied().unwrap_or(s10);
                let sum = s00 as u16 + s10 as u16 + s01 as u16 + s11 as u16;
                out.data_u[(y * out.stride_u + x) as usize] = (sum / 4) as u8;
            }
        }
        // Same for V
        for y in 0..out.chroma_height() {
            let sy = (y * 2) as usize;
            for x in 0..out.chroma_width() {
                let sx = (x * 2) as usize;
                let s00 = self.data_v[sy * self.stride_v as usize + sx];
                let s10 = self.data_v[sy * self.stride_v as usize + sx + 1];
                let s01 = self.data_v.get((sy + 1) * self.stride_v as usize + sx).copied().unwrap_or(s00);
                let s11 = self.data_v.get((sy + 1) * self.stride_v as usize + sx + 1).copied().unwrap_or(s10);
                let sum = s00 as u16 + s10 as u16 + s01 as u16 + s11 as u16;
                out.data_v[(y * out.stride_v + x) as usize] = (sum / 4) as u8;
            }
        }
        Ok(out)
    }
}

// ============================================================================
// NV12Buffer — biplanar 4:2:0 YUV, 8-bit
// ============================================================================

#[derive(Debug, Clone)]
pub struct NV12Buffer {
    pub width: u32,
    pub height: u32,
    pub stride_y: u32,
    pub stride_uv: u32,
    pub data_y: Vec<u8>,
    pub data_uv: Vec<u8>,
}

impl NV12Buffer {
    pub fn new(width: u32, height: u32) -> Self {
        let stride_y = width;
        let stride_uv = width + (width % 2);
        Self {
            width, height, stride_y, stride_uv,
            data_y: vec![0u8; (stride_y * height) as usize],
            data_uv: vec![128u8; (stride_uv * ((height + 1) / 2)) as usize],
        }
    }

    pub fn chroma_width(&self) -> u32 { (self.width + 1) / 2 }
    pub fn chroma_height(&self) -> u32 { (self.height + 1) / 2 }
}

impl VideoBuffer for NV12Buffer {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn buffer_type(&self) -> VideoBufferType { VideoBufferType::NV12 }
    fn as_nv12(&self) -> Option<&NV12Buffer> { Some(self) }

    fn scale(&self, w: u32, h: u32) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        self.to_i420()?.scale(w, h)
    }

    fn to_i420(&self) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        let mut out = I420Buffer::new(self.width, self.height);
        out.data_y.copy_from_slice(&self.data_y);
        for y in 0..out.chroma_height() {
            for x in 0..out.chroma_width() {
                let uv_off = ((y * self.stride_uv) + x * 2) as usize;
                let dst_off = (y * out.stride_u + x) as usize;
                out.data_u[dst_off] = self.data_uv[uv_off];
                out.data_v[dst_off] = self.data_uv[uv_off + 1];
            }
        }
        Ok(out)
    }
}

// ============================================================================
// I010Buffer — planar 4:2:0 YUV, 10-bit (stored as u16)
// ============================================================================

#[derive(Debug, Clone)]
pub struct I010Buffer {
    pub width: u32,
    pub height: u32,
    pub stride_y: u32,
    pub stride_u: u32,
    pub stride_v: u32,
    pub data_y: Vec<u16>,
    pub data_u: Vec<u16>,
    pub data_v: Vec<u16>,
}

impl I010Buffer {
    pub fn new(width: u32, height: u32) -> Self {
        let stride_y = width;
        let stride_u = (width + 1) / 2;
        let stride_v = (width + 1) / 2;
        Self {
            width, height, stride_y, stride_u, stride_v,
            data_y: vec![0u16; (stride_y * height) as usize],
            data_u: vec![512u16; (stride_u * ((height + 1) / 2)) as usize],
            data_v: vec![512u16; (stride_v * ((height + 1) / 2)) as usize],
        }
    }
}

impl VideoBuffer for I010Buffer {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn buffer_type(&self) -> VideoBufferType { VideoBufferType::I010 }
    fn as_i010(&self) -> Option<&I010Buffer> { Some(self) }

    fn scale(&self, w: u32, h: u32) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        self.to_i420()?.scale(w, h)
    }

    fn to_i420(&self) -> Result<I420Buffer, crate::protocols::rtc::peer::core::MediaError> {
        // 10-bit → 8-bit truncation + I420 layout
        let mut out = I420Buffer::new(self.width, self.height);
        for (i, v) in self.data_y.iter().enumerate() {
            out.data_y.get_mut(i).map(|b| *b = (*v >> 2) as u8);
        }
        for (i, v) in self.data_u.iter().enumerate() {
            out.data_u.get_mut(i).map(|b| *b = (*v >> 2) as u8);
        }
        for (i, v) in self.data_v.iter().enumerate() {
            out.data_v.get_mut(i).map(|b| *b = (*v >> 2) as u8);
        }
        Ok(out)
    }
}
