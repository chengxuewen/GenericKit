// Video frame transform tests (scale, crop, rotate).
// Pattern: WebRTC common_video/video_frame_unittest.cc — CropXCenter, Scale, Rotates

use gkit_media::video::buffer::{I420Buffer, VideoBuffer};
use gkit_media::video::transform::{i420_crop, i420_rotate, i420_scale};

// ============================================================================
// Scale
// ============================================================================

fn fill_gradient(buf: &mut I420Buffer) {
    for y in 0..buf.height {
        for x in 0..buf.width {
            let val = ((x + y) % 256) as u8;
            buf.data_y[(y * buf.stride_y + x) as usize] = val;
        }
    }
}

#[test]
fn scale_down_by_half() {
    let mut src = I420Buffer::new(200, 100);
    fill_gradient(&mut src);
    let result = i420_scale(&src, 100, 50).unwrap();
    assert_eq!(result.width, 100);
    assert_eq!(result.height, 50);
    assert_eq!(result.chroma_width(), 50);
    assert_eq!(result.chroma_height(), 25);
}

#[test]
fn scale_identity() {
    let mut src = I420Buffer::new(64, 64);
    fill_gradient(&mut src);
    let result = i420_scale(&src, 64, 64).unwrap();
    assert_eq!(result.width, 64);
    assert_eq!(result.height, 64);
}

#[test]
fn scale_up() {
    let mut src = I420Buffer::new(32, 32);
    fill_gradient(&mut src);
    let result = i420_scale(&src, 64, 64).unwrap();
    assert_eq!(result.width, 64);
    assert_eq!(result.height, 64);
}

// ============================================================================
// Crop
// ============================================================================

fn fill_sequential(buf: &mut I420Buffer) {
    // Fill Y plane with sequential values by column
    for y in 0..buf.height {
        for x in 0..buf.width {
            buf.data_y[(y * buf.stride_y + x) as usize] = ((x * 7 + y * 13) % 256) as u8;
        }
    }
    for y in 0..buf.chroma_height() {
        for x in 0..buf.chroma_width() {
            buf.data_u[(y * buf.stride_u + x) as usize] = ((x * 3 + y * 5) % 256) as u8;
            buf.data_v[(y * buf.stride_v + x) as usize] = ((x * 11 + y * 17) % 256) as u8;
        }
    }
}

#[test]
fn crop_center() {
    let mut src = I420Buffer::new(64, 64);
    fill_sequential(&mut src);

    // Crop 32x32 from center
    let result = i420_crop(&src, 16, 16, 32, 32).unwrap();
    assert_eq!(result.width, 32);
    assert_eq!(result.height, 32);
    assert_eq!(result.chroma_width(), 16);
    assert_eq!(result.chroma_height(), 16);
}

#[test]
fn crop_full_frame() {
    let mut src = I420Buffer::new(32, 32);
    fill_sequential(&mut src);
    let result = i420_crop(&src, 0, 0, 32, 32).unwrap();
    assert_eq!(result.width, 32);
    assert_eq!(result.height, 32);
}

#[test]
fn crop_alignment_required() {
    let src = I420Buffer::new(64, 64);
    // Odd coords should fail for I420
    assert!(i420_crop(&src, 1, 0, 8, 8).is_err()); // x is odd
    assert!(i420_crop(&src, 0, 1, 8, 8).is_err()); // y is odd
    assert!(i420_crop(&src, 0, 0, 9, 8).is_err()); // w is odd
}

#[test]
fn crop_out_of_bounds() {
    let src = I420Buffer::new(32, 32);
    assert!(i420_crop(&src, 0, 0, 64, 32).is_err());
    assert!(i420_crop(&src, 0, 0, 32, 64).is_err());
}

// ============================================================================
// Rotate
// ============================================================================

fn fill_test_pattern(buf: &mut I420Buffer) {
    // Fill top-left corner with known values for corner check
    for y in 0..buf.height {
        for x in 0..buf.width {
            let val = if x < 4 && y < 4 { 255 } else { 0 };
            buf.data_y[(y * buf.stride_y + x) as usize] = val as u8;
        }
    }
}

#[test]
fn rotate_0_is_identity() {
    let mut src = I420Buffer::new(16, 8);
    fill_test_pattern(&mut src);
    let result = i420_rotate(&src, 0).unwrap();
    assert_eq!(result.width, 16);
    assert_eq!(result.height, 8);
}

#[test]
fn rotate_180() {
    let mut src = I420Buffer::new(16, 8);
    fill_test_pattern(&mut src);
    let result = i420_rotate(&src, 180).unwrap();
    assert_eq!(result.width, 16);
    assert_eq!(result.height, 8);
    // Top-left of original should be at bottom-right of rotated
    assert_eq!(src.data_y[0], 255);
    let br_idx = ((result.height - 1) * result.stride_y + (result.width - 1)) as usize;
    assert_eq!(result.data_y[br_idx], 255);
}

#[test]
fn rotate_90() {
    let mut src = I420Buffer::new(16, 8);
    fill_test_pattern(&mut src);
    let result = i420_rotate(&src, 90).unwrap();
    // Width/height swap
    assert_eq!(result.width, 8);
    assert_eq!(result.height, 16);
}

#[test]
fn rotate_270() {
    let mut src = I420Buffer::new(8, 16);
    fill_test_pattern(&mut src);
    let result = i420_rotate(&src, 270).unwrap();
    assert_eq!(result.width, 16);
    assert_eq!(result.height, 8);
}

#[test]
fn rotate_360_is_identity() {
    let mut src = I420Buffer::new(16, 8);
    fill_test_pattern(&mut src);
    let result = i420_rotate(&src, 360).unwrap();
    assert_eq!(result.width, 16);
    assert_eq!(result.height, 8);
}

#[test]
fn rotate_invalid_angle() {
    let src = I420Buffer::new(16, 8);
    assert!(i420_rotate(&src, 45).is_err());
    assert!(i420_rotate(&src, 95).is_err());
}

// ============================================================================
// Combined operations
// ============================================================================

#[test]
fn crop_then_scale() {
    let mut src = I420Buffer::new(128, 128);
    fill_sequential(&mut src);
    let cropped = i420_crop(&src, 32, 32, 64, 64).unwrap();
    let scaled = i420_scale(&cropped, 32, 32).unwrap();
    assert_eq!(scaled.width, 32);
    assert_eq!(scaled.height, 32);
}

#[test]
fn rotate_then_scale() {
    let mut src = I420Buffer::new(64, 32);
    fill_sequential(&mut src);
    let rotated = i420_rotate(&src, 90).unwrap();
    let scaled = i420_scale(&rotated, rotated.width / 2, rotated.height / 2).unwrap();
    assert_eq!(scaled.width, 16);
    assert_eq!(scaled.height, 32);
}
