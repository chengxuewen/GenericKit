// VideoBuffer type tests.
// Pattern: WebRTC api/video/test/i420_buffer_unittest.cc, nv12_buffer_unittest.cc, etc.

use gkit_media::video::buffer::{
    I010Buffer, I420Buffer, I422Buffer, I444Buffer, NV12Buffer, VideoBuffer,
};

// ============================================================================
// I420Buffer
// ============================================================================

#[test]
fn i420_dimensions() {
    let buf = I420Buffer::new(320, 240);
    assert_eq!(buf.width, 320);
    assert_eq!(buf.height, 240);
    assert_eq!(buf.stride_y, 320);
    assert_eq!(buf.stride_u, 160);
    assert_eq!(buf.stride_v, 160);
    assert_eq!(buf.chroma_width(), 160);
    assert_eq!(buf.chroma_height(), 120);
}

#[test]
fn i420_plane_sizes() {
    let buf = I420Buffer::new(320, 240);
    let size_y = (buf.stride_y * buf.height) as usize;
    let size_u = (buf.stride_u * buf.chroma_height()) as usize;
    let size_v = (buf.stride_v * buf.chroma_height()) as usize;
    assert_eq!(buf.data_y.len(), size_y);
    assert_eq!(buf.data_u.len(), size_u);
    assert_eq!(buf.data_v.len(), size_v);
}

#[test]
fn i420_odd_dimensions() {
    let buf = I420Buffer::new(319, 239);
    assert_eq!(buf.stride_u, 160); // (319+1)/2
    assert_eq!(buf.chroma_width(), 160);
    assert_eq!(buf.chroma_height(), 120); // (239+1)/2
}

#[test]
fn i420_clone() {
    let mut buf = I420Buffer::new(10, 10);
    for i in 0..buf.data_y.len() {
        buf.data_y[i] = (i % 256) as u8;
    }
    let buf2 = buf.clone();
    assert_eq!(buf2.data_y, buf.data_y);
    assert_eq!(buf2.data_u, buf.data_u);
    assert_eq!(buf2.data_v, buf.data_v);
    assert_eq!(buf2.width, buf.width);
    assert_eq!(buf2.height, buf.height);
}

#[test]
fn i420_to_i420_is_identity() {
    let buf = I420Buffer::new(32, 32);
    let converted = buf.to_i420().unwrap();
    assert_eq!(converted.data_y, buf.data_y);
}

// ============================================================================
// NV12Buffer
// ============================================================================

#[test]
fn nv12_dimensions() {
    let buf = NV12Buffer::new(320, 240);
    assert_eq!(buf.width, 320);
    assert_eq!(buf.height, 240);
    assert_eq!(buf.stride_y, 320);
    assert_eq!(buf.stride_uv, 320); // width is even
    assert_eq!(buf.chroma_width(), 160);
    assert_eq!(buf.chroma_height(), 120);
}

#[test]
fn nv12_odd_width_stride() {
    let buf = NV12Buffer::new(319, 240);
    assert_eq!(buf.stride_y, 319);
    assert_eq!(buf.stride_uv, 320); // 319 + 319%2 = 320
}

#[test]
fn nv12_to_i420() {
    let mut buf = NV12Buffer::new(64, 64);
    // Fill with test pattern: Y=row index, UV=(column, row)
    for y in 0..buf.height {
        for x in 0..buf.width {
            buf.data_y[(y * buf.stride_y + x) as usize] = y as u8;
        }
    }
    for y in 0..buf.chroma_height() {
        for x in 0..buf.chroma_width() {
            let idx = (y * buf.stride_uv + x * 2) as usize;
            buf.data_uv[idx] = 128;     // U
            buf.data_uv[idx + 1] = 129; // V
        }
    }
    let i420 = buf.to_i420().unwrap();
    assert_eq!(i420.width, 64);
    assert_eq!(i420.height, 64);
}

// ============================================================================
// I422Buffer
// ============================================================================

#[test]
fn i422_dimensions() {
    let buf = I422Buffer::new(320, 240);
    assert_eq!(buf.stride_y, 320);
    assert_eq!(buf.stride_u, 160);
    // I422 chroma has full height (4:2:2)
    let size_u = (buf.stride_u * buf.height) as usize;
    assert_eq!(buf.data_u.len(), size_u);
}

#[test]
fn i422_to_i420() {
    let buf = I422Buffer::new(64, 64);
    let i420 = buf.to_i420().unwrap();
    assert_eq!(i420.width, 64);
    assert_eq!(i420.height, 64);
    assert_eq!(i420.chroma_height(), 32); // halved from 64
}

// ============================================================================
// I444Buffer
// ============================================================================

#[test]
fn i444_dimensions() {
    let buf = I444Buffer::new(320, 240);
    assert_eq!(buf.stride_y, 320);
    assert_eq!(buf.stride_u, 320); // full chroma stride
    assert_eq!(buf.stride_v, 320);
}

#[test]
fn i444_to_i420() {
    let buf = I444Buffer::new(64, 64);
    let i420 = buf.to_i420().unwrap();
    assert_eq!(i420.chroma_height(), 32); // quarter sampling
    assert_eq!(i420.chroma_width(), 32);
}

// ============================================================================
// I010Buffer
// ============================================================================

#[test]
fn i010_dimensions() {
    let buf = I010Buffer::new(320, 240);
    assert_eq!(buf.width, 320);
    assert_eq!(buf.height, 240);
}

#[test]
fn i010_to_i420() {
    let mut buf = I010Buffer::new(32, 32);
    // Fill with 10-bit mid-gray
    for v in buf.data_y.iter_mut() {
        *v = 512;
    }
    let i420 = buf.to_i420().unwrap();
    assert_eq!(i420.width, 32);
    assert_eq!(i420.height, 32);
    // 10-bit 512 >> 2 = 128
    assert_eq!(i420.data_y[0], 128);
}

// ============================================================================
// VideoBuffer type helpers
// ============================================================================

#[test]
fn buffer_as_cast() {
    let buf = I420Buffer::new(10, 10);
    assert!(buf.as_i420().is_some());
    assert!(buf.as_nv12().is_none());
    assert!(buf.as_i422().is_none());
    assert!(buf.as_i444().is_none());
    assert!(buf.as_i010().is_none());
}

#[test]
fn nv12_as_cast() {
    let buf = NV12Buffer::new(10, 10);
    assert!(buf.as_nv12().is_some());
    assert!(buf.as_i420().is_none());
}

#[test]
fn extreme_small_dimensions() {
    let buf = I420Buffer::new(2, 2);
    assert_eq!(buf.width, 2);
    assert_eq!(buf.height, 2);
    assert_eq!(buf.chroma_width(), 1);
    assert_eq!(buf.chroma_height(), 1);
}

#[test]
fn extreme_large_dimensions() {
    let buf = I420Buffer::new(3840, 2160); // 4K
    assert_eq!(buf.width, 3840);
    assert_eq!(buf.height, 2160);
    assert_eq!(buf.chroma_width(), 1920);
    assert_eq!(buf.stride_y, 3840);
}
