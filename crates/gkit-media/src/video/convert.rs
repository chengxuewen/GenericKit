use crate::protocols::rtc::client::core::MediaError;

use super::buffer::{
    I420Buffer, I422Buffer, I444Buffer, NV12Buffer, VideoBuffer, VideoBufferType, VideoFormatType,
};

/// Convert raw RGBA pixels to I420 using BT.601 full range.
pub fn argb_to_i420(rgba: &[u8], width: u32, height: u32, stride: u32) -> Result<I420Buffer, MediaError> {
    let mut i420 = I420Buffer::new(width, height);
    let w = width as usize;
    let h = height as usize;
    let mut full_u = vec![0u8; w * h];
    let mut full_v = vec![0u8; w * h];

    for y in 0..h {
        for x in 0..w {
            let i = (y * stride as usize + x * 4) as usize;
            let r = rgba[i] as f64;
            let g = rgba[i + 1] as f64;
            let b = rgba[i + 2] as f64;
            let yv = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
            let uv = (-0.14713 * r - 0.28886 * g + 0.436 * b + 128.0).clamp(0.0, 255.0) as u8;
            let vv = (0.615 * r - 0.51499 * g - 0.10001 * b + 128.0).clamp(0.0, 255.0) as u8;
            let idx = y * w + x;
            i420.data_y[idx] = yv;
            full_u[idx] = uv;
            full_v[idx] = vv;
        }
    }

    // 4:4:4 → 4:2:0 chroma subsampling (2×2 average)
    let cw = i420.chroma_width() as usize;
    let ch = i420.chroma_height() as usize;
    for cy in 0..ch {
        for cx in 0..cw {
            let x0 = cx * 2;
            let y0 = cy * 2;
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);
            let u = (full_u[y0 * w + x0] as u16 + full_u[y0 * w + x1] as u16
                + full_u[y1 * w + x0] as u16 + full_u[y1 * w + x1] as u16) / 4;
            let v = (full_v[y0 * w + x0] as u16 + full_v[y0 * w + x1] as u16
                + full_v[y1 * w + x0] as u16 + full_v[y1 * w + x1] as u16) / 4;
            i420.data_u[cy * i420.stride_u as usize + cx] = u as u8;
            i420.data_v[cy * i420.stride_v as usize + cx] = v as u8;
        }
    }
    Ok(i420)
}

/// Convert I420 to RGBA (BT.601 full range).
pub fn i420_to_argb(i420: &I420Buffer, out: &mut [u8], out_stride: u32, format: VideoFormatType) {
    for y in 0..i420.height {
        for x in 0..i420.width {
            let yi = (y * i420.stride_y + x) as usize;
            let uv_y = y / 2;
            let uv_x = x / 2;
            let ui = (uv_y * i420.stride_u + uv_x) as usize;
            let vi = (uv_y * i420.stride_v + uv_x) as usize;

            let yy = i420.data_y[yi] as f64;
            let uu = i420.data_u[ui] as f64 - 128.0;
            let vv = i420.data_v[vi] as f64 - 128.0;

            let r = (yy + 1.402 * vv).clamp(0.0, 255.0) as u8;
            let g = (yy - 0.344 * uu - 0.714 * vv).clamp(0.0, 255.0) as u8;
            let b = (yy + 1.772 * uu).clamp(0.0, 255.0) as u8;

            let oi = (y * out_stride + x * 4) as usize;
            match format {
                VideoFormatType::ARGB => { out[oi] = 255; out[oi+1] = r; out[oi+2] = g; out[oi+3] = b; }
                VideoFormatType::BGRA => { out[oi] = b; out[oi+1] = g; out[oi+2] = r; out[oi+3] = 255; }
                VideoFormatType::ABGR => { out[oi] = 255; out[oi+1] = b; out[oi+2] = g; out[oi+3] = r; }
                VideoFormatType::RGBA => { out[oi] = r; out[oi+1] = g; out[oi+2] = b; out[oi+3] = 255; }
            }
        }
    }
}

/// Convert I420 to packed RGB24 (R-G-B, 3 bytes per pixel).
pub fn i420_to_rgb24(i420: &I420Buffer, out: &mut [u8]) {
    for y in 0..i420.height {
        for x in 0..i420.width {
            let yi = (y * i420.stride_y + x) as usize;
            let uv_y = y / 2;
            let uv_x = x / 2;
            let ui = (uv_y * i420.stride_u + uv_x) as usize;
            let vi = (uv_y * i420.stride_v + uv_x) as usize;

            let yy = i420.data_y[yi] as f64;
            let uu = i420.data_u[ui] as f64 - 128.0;
            let vv = i420.data_v[vi] as f64 - 128.0;

            let r = (yy + 1.402 * vv).clamp(0.0, 255.0) as u8;
            let g = (yy - 0.344 * uu - 0.714 * vv).clamp(0.0, 255.0) as u8;
            let b = (yy + 1.772 * uu).clamp(0.0, 255.0) as u8;

            let oi = ((y * i420.width + x) * 3) as usize;
            out[oi] = r;
            out[oi + 1] = g;
            out[oi + 2] = b;
        }
    }
}

// ============================================================================
// I420 ↔ NV12 / NV21
// ============================================================================

pub fn i420_to_nv12(i420: &I420Buffer) -> NV12Buffer {
    let mut nv12 = NV12Buffer::new(i420.width, i420.height);
    nv12.data_y.copy_from_slice(&i420.data_y);
    let cw = nv12.chroma_width() as usize;
    let ch = nv12.chroma_height() as usize;
    for y in 0..ch {
        for x in 0..cw {
            let off = (y * nv12.stride_uv as usize) + x * 2;
            let src = y * i420.stride_u as usize + x;
            nv12.data_uv[off] = i420.data_u[src];
            nv12.data_uv[off + 1] = i420.data_v[src];
        }
    }
    nv12
}

pub fn i420_to_nv21(i420: &I420Buffer) -> NV12Buffer {
    let mut nv21 = NV12Buffer::new(i420.width, i420.height);
    nv21.data_y.copy_from_slice(&i420.data_y);
    let cw = nv21.chroma_width() as usize;
    let ch = nv21.chroma_height() as usize;
    for y in 0..ch {
        for x in 0..cw {
            let off = (y * nv21.stride_uv as usize) + x * 2;
            let src = y * i420.stride_u as usize + x;
            nv21.data_uv[off] = i420.data_v[src];     // V first (NV21 order)
            nv21.data_uv[off + 1] = i420.data_u[src];  // U second
        }
    }
    nv21
}

/// Convert NV21 (V-U order) back to I420 correctly.
/// The default NV12Buffer::to_i420 reads U-V order, so we swap U/V in the result.
pub fn nv21_to_i420(nv21: &NV12Buffer) -> Result<I420Buffer, MediaError> {
    let mut i420 = nv21.to_i420()?;
    // NV12 to_i420 assumed U-V order, but data was V-U; swap U/V planes
    std::mem::swap(&mut i420.data_u, &mut i420.data_v);
    Ok(i420)
}

// ============================================================================
// I420 ↔ packed YUV (YUY2 / UYVY)
// ============================================================================

pub fn i420_to_yuy2(i420: &I420Buffer, out: &mut [u8]) {
    for y in 0..i420.height {
        for x in (0..i420.width).step_by(2) {
            let y0 = (y * i420.stride_y + x) as usize;
            let y1 = (y * i420.stride_y + x + 1) as usize;
            let uv_x = (x / 2) as usize;
            let uv_y = (y / 2) as usize;
            let u = (uv_y * i420.stride_u as usize + uv_x) as usize;
            let v = (uv_y * i420.stride_v as usize + uv_x) as usize;
            let oi = ((y * i420.width + x) * 2) as usize;
            out[oi] = i420.data_y[y0];
            out[oi + 1] = i420.data_u[u];
            out[oi + 2] = i420.data_y[y1];
            out[oi + 3] = i420.data_v[v];
        }
    }
}

pub fn i420_to_uyvy(i420: &I420Buffer, out: &mut [u8]) {
    for y in 0..i420.height {
        for x in (0..i420.width).step_by(2) {
            let y0 = (y * i420.stride_y + x) as usize;
            let y1 = (y * i420.stride_y + x + 1) as usize;
            let uv_x = (x / 2) as usize;
            let uv_y = (y / 2) as usize;
            let u = (uv_y * i420.stride_u as usize + uv_x) as usize;
            let v = (uv_y * i420.stride_v as usize + uv_x) as usize;
            let oi = ((y * i420.width + x) * 2) as usize;
            out[oi] = i420.data_u[u];
            out[oi + 1] = i420.data_y[y0];
            out[oi + 2] = i420.data_v[v];
            out[oi + 3] = i420.data_y[y1];
        }
    }
}

// ============================================================================
// Generic to_i420
// ============================================================================

pub fn to_i420(width: u32, height: u32, src_type: VideoBufferType,
               data_y: &[u8], stride_y: u32,
               data_u: &[u8], stride_u: u32,
                data_v: &[u8], _stride_v: u32,
               _uv: &[u8], _stride_uv: u32,
) -> Result<I420Buffer, MediaError> {
    match src_type {
        VideoBufferType::I420 | VideoBufferType::I420A => {
            let mut buf = I420Buffer::new(width, height);
            let sw = width as usize;
            for y in 0..height {
                let y = y as usize;
                let sy = (y as u32 * stride_y) as usize;
                let dy = (y as u32 * buf.stride_y) as usize;
                buf.data_y[dy..dy+sw].copy_from_slice(&data_y[sy..sy+sw]);
            }
            let ch = ((height + 1) / 2) as usize;
            let cw2 = ((width + 1) / 2) as usize;
            for y in 0..ch {
                let sy = (y as u32 * stride_u) as usize;
                let dy = (y as u32 * buf.stride_u) as usize;
                buf.data_u[dy..dy+cw2].copy_from_slice(&data_u[sy..sy+cw2]);
                buf.data_v[dy..dy+cw2].copy_from_slice(&data_v[sy..sy+cw2]);
            }
            Ok(buf)
        }
        VideoBufferType::I422 => I422Buffer::new(width, height).to_i420(),
        VideoBufferType::I444 => I444Buffer::new(width, height).to_i420(),
        VideoBufferType::NV12 => NV12Buffer::new(width, height).to_i420(),
        _ => Err(MediaError::new("unsupported buffer type")),
    }
}
