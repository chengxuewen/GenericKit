use crate::protocols::rtc::client::core::MediaError;

use super::buffer::{I420Buffer, VideoBuffer};

/// Scale an I420 buffer to new dimensions using bilinear interpolation.
pub fn i420_scale(i420: &I420Buffer, scaled_width: u32, scaled_height: u32) -> Result<I420Buffer, MediaError> {
    i420.scale(scaled_width, scaled_height)
}

/// Crop a region from an I420 buffer.
/// x,y,w,h must be even-aligned for I420 chroma subsampling.
pub fn i420_crop(i420: &I420Buffer, x: u32, y: u32, w: u32, h: u32) -> Result<I420Buffer, MediaError> {
    if x % 2 != 0 || y % 2 != 0 || w % 2 != 0 || h % 2 != 0 {
        return Err(MediaError::new("crop coordinates must be even-aligned for I420"));
    }
    if x + w > i420.width || y + h > i420.height {
        return Err(MediaError::new("crop region exceeds buffer dimensions"));
    }

    let mut out = I420Buffer::new(w, h);
    // Copy Y plane
    for row in 0..h {
        let src_off = ((y + row) * i420.stride_y + x) as usize;
        let dst_off = (row * out.stride_y) as usize;
        out.data_y[dst_off..dst_off + w as usize]
            .copy_from_slice(&i420.data_y[src_off..src_off + w as usize]);
    }
    // Copy U/V planes (half resolution)
    let cx = x / 2;
    let cy = y / 2;
    let cw = w / 2;
    let ch = h / 2;
    for row in 0..ch {
        let src_u = ((cy + row) * i420.stride_u + cx) as usize;
        let dst_u = (row * out.stride_u) as usize;
        out.data_u[dst_u..dst_u + cw as usize].copy_from_slice(&i420.data_u[src_u..src_u + cw as usize]);
        out.data_v[dst_u..dst_u + cw as usize].copy_from_slice(&i420.data_v[src_u..src_u + cw as usize]);
    }
    Ok(out)
}

/// Rotate I420 by 0, 90, 180, or 270 degrees.
pub fn i420_rotate(i420: &I420Buffer, degrees: u32) -> Result<I420Buffer, MediaError> {
    match degrees % 360 {
        0 => Ok(i420.clone()),
        90 => rotate90(i420),
        180 => rotate180(i420),
        270 => rotate270(i420),
        _ => Err(MediaError::new("rotation must be 0, 90, 180, or 270 degrees")),
    }
}

fn rotate180(i420: &I420Buffer) -> Result<I420Buffer, MediaError> {
    let mut out = I420Buffer::new(i420.width, i420.height);
    // Y: reverse rows and columns
    for y in 0..i420.height {
        for x in 0..i420.width {
            let src = ((i420.height - 1 - y) * i420.stride_y + (i420.width - 1 - x)) as usize;
            let dst = (y * out.stride_y + x) as usize;
            out.data_y[dst] = i420.data_y[src];
        }
    }
    let cw = i420.chroma_width();
    let ch = i420.chroma_height();
    for y in 0..ch {
        for x in 0..cw {
            let su = ((ch - 1 - y) * i420.stride_u + (cw - 1 - x)) as usize;
            let du = (y * out.stride_u + x) as usize;
            out.data_u[du] = i420.data_u[su];
            out.data_v[du] = i420.data_v[su];
        }
    }
    Ok(out)
}

fn rotate90(i420: &I420Buffer) -> Result<I420Buffer, MediaError> {
    // Width and height swap for 90° rotation
    let out_w = i420.height;
    let out_h = i420.width;
    let mut out = I420Buffer::new(out_w, out_h);
    for y in 0..out_h {
        for x in 0..out_w {
            let src = ((out_w - 1 - x) * i420.stride_y + y) as usize;
            let dst = (y * out.stride_y + x) as usize;
            out.data_y[dst] = i420.data_y[src];
        }
    }
    let cw = out.chroma_width();
    let ch = out.chroma_height();
    let _scw = i420.chroma_width();
    let sch = i420.chroma_height();
    for y in 0..ch {
        for x in 0..cw {
            let su = ((sch - 1 - x) * i420.stride_u + y) as usize;
            let du = (y * out.stride_u + x) as usize;
            out.data_u[du] = i420.data_u[su];
            out.data_v[du] = i420.data_v[su];
        }
    }
    Ok(out)
}

fn rotate270(i420: &I420Buffer) -> Result<I420Buffer, MediaError> {
    let out_w = i420.height;
    let out_h = i420.width;
    let mut out = I420Buffer::new(out_w, out_h);
    for y in 0..out_h {
        for x in 0..out_w {
            let src = (x * i420.stride_y + (out_h - 1 - y)) as usize;
            let dst = (y * out.stride_y + x) as usize;
            out.data_y[dst] = i420.data_y[src];
        }
    }
    let cw = out.chroma_width();
    let ch = out.chroma_height();
    let _scw = i420.chroma_width();
    for y in 0..ch {
        for x in 0..cw {
            let su = (x * i420.stride_u + (ch - 1 - y)) as usize;
            let du = (y * out.stride_u + x) as usize;
            out.data_u[du] = i420.data_u[su];
            out.data_v[du] = i420.data_v[su];
        }
    }
    Ok(out)
}
