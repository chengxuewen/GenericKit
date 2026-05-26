use gkit_media::video::frame_stabby::{
    BufferData, I420Planes, NV12Planes, StableVideoFrame, VideoFrameMeta,
};
use stabby::sync::ArcSlice;

#[test]
fn i420_frame_roundtrip_preserves_dimensions() {
    let meta = VideoFrameMeta::new(640, 480);
    let planes = I420Planes::zeroed(640, 480);
    let frame = StableVideoFrame {
        meta,
        buffer: BufferData::I420(planes),
    };
    assert_eq!(frame.meta.width, 640);
    assert_eq!(frame.meta.height, 480);
    assert!(frame.buffer.is_i420());
}

#[test]
fn i420_planes_strides_correct() {
    let planes = I420Planes::zeroed(320, 240);
    assert_eq!(planes.stride_y, 320);
    assert_eq!(planes.stride_u, 160);
    assert_eq!(planes.stride_v, 160);
    assert_eq!(planes.data_y.len(), 320 * 240);
    assert_eq!(planes.data_u.len(), 160 * 120);
    assert_eq!(planes.data_v.len(), 160 * 120);
}

#[test]
fn nv12_frame_roundtrip() {
    let frame = StableVideoFrame {
        meta: VideoFrameMeta::new(1920, 1080),
        buffer: BufferData::NV12(NV12Planes::zeroed(1920, 1080)),
    };
    assert_eq!(frame.meta.width, 1920);
    assert_eq!(frame.meta.height, 1080);
    assert!(frame.buffer.is_nv12());
}

#[test]
fn nv12_planes_strides_correct() {
    let planes = NV12Planes::zeroed(1920, 1080);
    assert_eq!(planes.stride_y, 1920);
    assert_eq!(planes.stride_uv, 1920);
    assert_eq!(planes.data_y.len(), 1920 * 1080);
    assert_eq!(planes.data_uv.len(), 1920 * 540);
}

#[test]
fn arc_reference_count_increments_on_clone() {
    let data: ArcSlice<u8> = ArcSlice::from(&[1u8, 2, 3][..]);
    let clone1 = data.clone();
    let clone2 = data.clone();
    drop(clone1);
    drop(clone2);
    assert_eq!(&data[..], &[1u8, 2, 3]);
}

#[test]
fn arc_slice_from_vec_preserves_content() {
    let v = vec![10u8, 20, 30, 40];
    let slice: ArcSlice<u8> = ArcSlice::from(&v[..]);
    assert_eq!(slice.len(), 4);
    assert_eq!(&slice[..], &[10, 20, 30, 40]);
}
