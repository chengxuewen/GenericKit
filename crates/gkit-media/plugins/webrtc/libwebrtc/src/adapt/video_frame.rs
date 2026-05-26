use gkit_media::video::buffer as gkit_buf;
use gkit_media::video::frame as gkit_frame;
use libwebrtc::video_frame as lk_vf;
use lk_vf::VideoBuffer as _;

// ---------------------------------------------------------------------------
// Rotation conversions
// ---------------------------------------------------------------------------



// ---------------------------------------------------------------------------
// I420 buffer conversions (both directions)
// ---------------------------------------------------------------------------

/// Create a libwebrtc `I420Buffer` from gkit's owned `I420Buffer`.
pub fn gkit_i420_to_lk(src: &gkit_buf::I420Buffer) -> lk_vf::I420Buffer {
    let mut lk_buf = lk_vf::I420Buffer::with_strides(
        src.width,
        src.height,
        src.stride_y,
        src.stride_u,
        src.stride_v,
    );
    let (y, u, v) = lk_buf.data_mut();
    y.copy_from_slice(&src.data_y);
    u.copy_from_slice(&src.data_u);
    v.copy_from_slice(&src.data_v);
    lk_buf
}

/// Create a gkit `I420Buffer` from a libwebrtc `I420Buffer` by copying data.
pub fn gkit_i420_from_lk(lk: &lk_vf::I420Buffer) -> gkit_buf::I420Buffer {
    let (y, u, v) = lk.data();
    let (sy, su, sv) = lk.strides();
    gkit_buf::I420Buffer {
        width: lk.width(),
        height: lk.height(),
        stride_y: sy,
        stride_u: su,
        stride_v: sv,
        data_y: y.to_vec(),
        data_u: u.to_vec(),
        data_v: v.to_vec(),
    }
}

// ---------------------------------------------------------------------------
// BoxVideoFrame conversions (gkit ↔ libwebrtc)
// ---------------------------------------------------------------------------

/// Convert a gkit `BoxVideoFrame` into a libwebrtc `BoxVideoFrame`.
///
/// Currently supports I420 only. Other buffer types panic with a message.
pub fn gkit_box_frame_to_lk(
    frame: &gkit_frame::BoxVideoFrame,
) -> lk_vf::BoxVideoFrame {
    let lk_buffer: lk_vf::BoxVideoBuffer = match frame.buffer.buffer_type() {
        gkit_buf::VideoBufferType::I420 => {
            let i420 = frame.buffer.as_i420().expect("as_i420");
            Box::new(gkit_i420_to_lk(i420))
        }
        gkit_buf::VideoBufferType::I420A => {
            // I420A: drop alpha channel, convert as I420
            // TODO: support alpha plane when libwebrtc I420ABuffer is needed
            let i420 = frame.buffer.to_i420().expect("to_i420");
            Box::new(gkit_i420_to_lk(&i420))
        }
        gkit_buf::VideoBufferType::I422 => {
            let i420 = frame.buffer.to_i420().expect("to_i420");
            Box::new(gkit_i420_to_lk(&i420))
        }
        gkit_buf::VideoBufferType::I444 => {
            let i420 = frame.buffer.to_i420().expect("to_i420");
            Box::new(gkit_i420_to_lk(&i420))
        }
        gkit_buf::VideoBufferType::I010 => {
            let i420 = frame.buffer.to_i420().expect("to_i420");
            Box::new(gkit_i420_to_lk(&i420))
        }
        gkit_buf::VideoBufferType::NV12 => {
            let i420 = frame.buffer.to_i420().expect("to_i420");
            Box::new(gkit_i420_to_lk(&i420))
        }
        gkit_buf::VideoBufferType::Native => {
            panic!("gkit_frame_to_libwebrtc: Native buffer not supported in livekit_rs adapter");
        }
    };

    lk_vf::VideoFrame {
        rotation: crate::adapt::convert::gkit_rotation_to_lk(frame.rotation),
        timestamp_us: frame.timestamp_us,
        frame_metadata: None,
        buffer: lk_buffer,
    }
}

/// Convert a libwebrtc `BoxVideoFrame` into a gkit `BoxVideoFrame`.
///
/// Currently supports I420 only. Other buffer types panic with a message.
pub fn gkit_box_frame_from_lk(
    lk_frame: &lk_vf::BoxVideoFrame,
) -> gkit_frame::BoxVideoFrame {
    let gkit_buffer: Box<dyn gkit_buf::VideoBuffer> = match lk_frame.buffer.buffer_type() {
        lk_vf::VideoBufferType::I420 => {
            let i420 = lk_frame
                .buffer
                .as_i420()
                .expect("as_i420 on I420 buffer");
            Box::new(gkit_i420_from_lk(i420))
        }
        lk_vf::VideoBufferType::I420A => {
            // TODO: I420A buffer — convert with alpha plane
            let i420 = lk_frame
                .buffer
                .as_ref()
                .to_i420();
            Box::new(gkit_i420_from_lk(&i420))
        }
        lk_vf::VideoBufferType::I422 => {
            let (y, u, v) = lk_frame
                .buffer
                .as_i422()
                .map(|b| (b.data().0.to_vec(), b.data().1.to_vec(), b.data().2.to_vec()))
                .unwrap_or_else(|| {
                    let i420 = lk_frame.buffer.as_ref().to_i420();
                    (i420.data().0.to_vec(), i420.data().1.to_vec(), i420.data().2.to_vec())
                });
            let (sy, su, sv) = lk_frame
                .buffer
                .as_i422()
                .map(|b| b.strides())
                .unwrap_or_else(|| {
                    let i420 = lk_frame.buffer.as_ref().to_i420();
                    i420.strides()
                });
            let w = lk_frame.buffer.width();
            let h = lk_frame.buffer.height();
            Box::new(gkit_buf::I422Buffer {
                width: w, height: h, stride_y: sy, stride_u: su, stride_v: sv,
                data_y: y, data_u: u, data_v: v,
            })
        }
        lk_vf::VideoBufferType::I444 => {
            // TODO: I444 buffer — proper conversion
            let i420 = lk_frame.buffer.as_ref().to_i420();
            Box::new(gkit_i420_from_lk(&i420))
        }
        lk_vf::VideoBufferType::I010 => {
            // TODO: I010 buffer — 10-bit support
            let i420 = lk_frame.buffer.as_ref().to_i420();
            Box::new(gkit_i420_from_lk(&i420))
        }
        lk_vf::VideoBufferType::NV12 => {
            // TODO: NV12 buffer — proper conversion
            let i420 = lk_frame.buffer.as_ref().to_i420();
            Box::new(gkit_i420_from_lk(&i420))
        }
        _ => {
            panic!("gkit_box_frame_from_lk: unsupported buffer type");
        }
    };

    gkit_frame::VideoFrame {
        rotation: crate::adapt::convert::lk_rotation_to_gkit(lk_frame.rotation),
        timestamp_us: lk_frame.timestamp_us,
        metadata: None,
        buffer: gkit_buffer,
    }
}
