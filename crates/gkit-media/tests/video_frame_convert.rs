// Video format conversion tests.
// Pattern: WebRTC yuv_helper conversion tests

use gkit_media::video::buffer::{
    I420Buffer, I422Buffer, I444Buffer, NV12Buffer, VideoBuffer, VideoBufferType, VideoFormatType,
};
use gkit_media::video::convert::{i420_to_argb, to_i420};

// ============================================================================
// to_i420 — format → I420 conversion
// ============================================================================

#[test]
fn convert_i420_is_identity() {
    let mut buf = I420Buffer::new(16, 16);
    for (i, v) in buf.data_y.iter_mut().enumerate() { *v = (i % 256) as u8; }
    let result = to_i420(buf.width, buf.height, VideoBufferType::I420,
        &buf.data_y, buf.stride_y,
        &buf.data_u, buf.stride_u,
        &buf.data_v, buf.stride_v,
        &[], 0).unwrap();
    assert_eq!(result.data_y, buf.data_y);
}

#[test]
fn convert_nv12_to_i420() {
    let nv12 = NV12Buffer::new(64, 64);
    let result = to_i420(nv12.width, nv12.height, VideoBufferType::NV12,
        &nv12.data_y, nv12.stride_y,
        &[], 0, &[], 0,
        &nv12.data_uv, nv12.stride_uv).unwrap();
    assert_eq!(result.width, 64);
    assert_eq!(result.height, 64);
    assert_eq!(result.chroma_width(), 32);
}

#[test]
fn convert_i422_to_i420() {
    let i422 = I422Buffer::new(32, 16);
    let result = to_i420(i422.width, i422.height, VideoBufferType::I422,
        &i422.data_y, i422.stride_y,
        &i422.data_u, i422.stride_u,
        &i422.data_v, i422.stride_v,
        &[], 0).unwrap();
    assert_eq!(result.chroma_height(), 8); // halved
}

#[test]
fn convert_i444_to_i420() {
    let i444 = I444Buffer::new(32, 32);
    let result = to_i420(i444.width, i444.height, VideoBufferType::I444,
        &i444.data_y, i444.stride_y,
        &i444.data_u, i444.stride_u,
        &i444.data_v, i444.stride_v,
        &[], 0).unwrap();
    assert_eq!(result.chroma_width(), 16); // quartered
    assert_eq!(result.chroma_height(), 16);
}

// ============================================================================
// i420_to_argb — YUV → RGBA conversion
// ============================================================================

fn fill_i420_test_pattern(buf: &mut I420Buffer) {
    // Red region (top-left)
    for y in 0..4 {
        for x in 0..4 {
            buf.data_y[(y * buf.stride_y + x) as usize] = 76;  // Y for red
        }
    }
    buf.data_u[0] = 85;  // U for red
    buf.data_v[0] = 255; // V for red
}

#[test]
fn i420_to_argb_format() {
    let mut buf = I420Buffer::new(4, 4);
    fill_i420_test_pattern(&mut buf);
    let out_size = (4 * 4 * 4) as usize;
    let mut out = vec![0u8; out_size];
    i420_to_argb(&buf, &mut out, 4 * 4, VideoFormatType::ARGB);

    // ARGB: A should be 255
    assert!(out[0] > 0);  // alpha = 255
}

#[test]
fn i420_to_bgra_format() {
    let mut buf = I420Buffer::new(4, 4);
    fill_i420_test_pattern(&mut buf);
    let out_size = (4 * 4 * 4) as usize;
    let mut out = vec![0u8; out_size];
    i420_to_argb(&buf, &mut out, 4 * 4, VideoFormatType::BGRA);

    // BGRA: last byte is alpha
    assert_eq!(out[3], 255); // alpha
}

#[test]
fn i420_to_argb_all_formats_output_size() {
    let buf = I420Buffer::new(8, 8);
    let size = (8 * 8 * 4) as usize;
    for format in [VideoFormatType::ARGB, VideoFormatType::BGRA,
                   VideoFormatType::ABGR, VideoFormatType::RGBA].iter() {
        let mut out = vec![0u8; size];
        i420_to_argb(&buf, &mut out, 8 * 4, *format);
        // No panic = success
    }
}
