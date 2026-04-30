/// Video frame module.
///
/// Architecture:
///   frame.rs     — VideoFrame, VideoRotation, BoxVideoFrame types
///   buffer.rs    — VideoBuffer trait + concrete buffer types
///   convert.rs   — YUV format conversion (to_i420, to_nv12, to_argb)
///   transform.rs — Scale, crop, rotate operations
pub mod buffer;
pub mod convert;
pub mod frame;
pub mod transform;
