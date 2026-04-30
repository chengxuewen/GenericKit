// VideoFrame construction and property tests.
// Pattern: WebRTC common_video/video_frame_unittest.cc — WidthHeightValues, ShallowCopy

use gkit_media::video::buffer::{I420Buffer, NV12Buffer, VideoBuffer, VideoBufferType};
use gkit_media::video::frame::{FrameMetadata, VideoFrame, VideoRotation};

#[test]
fn construct_default() {
    let buf = I420Buffer::new(320, 240);
    let frame = VideoFrame::new(buf);
    assert_eq!(frame.buffer.width, 320);
    assert_eq!(frame.buffer.height, 240);
    assert_eq!(frame.rotation, VideoRotation::Rotation0);
    assert_eq!(frame.timestamp_us, 0);
    assert!(frame.metadata.is_none());
}

#[test]
fn construct_with_rotation() {
    let buf = I420Buffer::new(640, 480);
    let frame = VideoFrame::new(buf)
        .with_rotation(VideoRotation::Rotation90)
        .with_timestamp(42_000);

    assert_eq!(frame.rotation, VideoRotation::Rotation90);
    assert_eq!(frame.timestamp_us, 42_000);
    assert_eq!(frame.buffer.width, 640);
    assert_eq!(frame.buffer.height, 480);
}

#[test]
fn construct_with_metadata() {
    let buf = I420Buffer::new(100, 100);
    let meta = FrameMetadata {
        user_timestamp: Some(12345),
        frame_id: Some(7),
    };
    let frame = VideoFrame::new(buf).with_metadata(meta.clone());

    let m = frame.metadata.as_ref().unwrap();
    assert_eq!(m.user_timestamp, Some(12345));
    assert_eq!(m.frame_id, Some(7));
}

#[test]
fn frame_buffer_type() {
    let i420 = I420Buffer::new(10, 10);
    assert_eq!(i420.buffer_type(), VideoBufferType::I420);
    assert_eq!(i420.width(), 10);
    assert_eq!(i420.height(), 10);

    let nv12 = NV12Buffer::new(20, 30);
    assert_eq!(nv12.buffer_type(), VideoBufferType::NV12);
    assert_eq!(nv12.width(), 20);
    assert_eq!(nv12.height(), 30);
}

#[test]
fn frame_metadata_default() {
    let meta = FrameMetadata::default();
    assert_eq!(meta.user_timestamp, None);
    assert_eq!(meta.frame_id, None);
}

#[test]
fn video_rotation_values() {
    assert_eq!(VideoRotation::Rotation0 as i32, 0);
    assert_eq!(VideoRotation::Rotation90 as i32, 90);
    assert_eq!(VideoRotation::Rotation180 as i32, 180);
    assert_eq!(VideoRotation::Rotation270 as i32, 270);
}
