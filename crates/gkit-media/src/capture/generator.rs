use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::video::buffer::{I420Buffer, VideoBuffer};
use crate::video::frame::VideoFrame;
use crate::video::source_sink::{
    VideoSink, VideoSource, VideoSinkWants, VideoBroadcaster,
};

pub trait FramePattern: Send {
    fn draw(&mut self, y: &mut [u8], u: &mut [u8], v: &mut [u8],
            stride_y: u32, stride_u: u32, stride_v: u32);
}

pub struct SquarePattern {
    squares: Vec<Square>,
}

struct Square {
    x: u32,
    y: u32,
    size: u32,
    color_y: u8,
    color_u: u8,
    color_v: u8,
}

fn fast_rand_u32() -> u32 {
    thread_local! {
        static SEED: std::cell::Cell<u64> = std::cell::Cell::new(0x0123_4567_89AB_CDEF);
    }
    SEED.with(|seed| {
        let mut x = seed.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        seed.set(x);
        x as u32
    })
}

impl SquarePattern {
    pub fn new(width: u32, height: u32, num_squares: u32) -> Self {
        let mut squares = Vec::new();
        for _ in 0..num_squares {
            squares.push(Square {
                x: fast_rand_u32() % width,
                y: fast_rand_u32() % height,
                size: (fast_rand_u32() % (width.min(height) / 4)) + 4,
                color_y: fast_rand_u32() as u8,
                color_u: fast_rand_u32() as u8,
                color_v: fast_rand_u32() as u8,
            });
        }
        Self { squares }
    }
}

impl FramePattern for SquarePattern {
    fn draw(&mut self, y: &mut [u8], u: &mut [u8], v: &mut [u8],
            stride_y: u32, stride_u: u32, stride_v: u32)
    {
        let width = stride_y;
        let height = y.len() as u32 / stride_y;

        for row in y.chunks_mut(stride_y as usize) {
            row.fill(127);
        }
        for row in u.chunks_mut(stride_u as usize) {
            row.fill(127);
        }
        for row in v.chunks_mut(stride_v as usize) {
            row.fill(127);
        }

        for sq in &mut self.squares {
            draw_rect(y, stride_y, sq.x, sq.y, sq.size, sq.size, sq.color_y);
            draw_rect(u, stride_u, sq.x / 2, sq.y / 2, sq.size / 2, sq.size / 2, sq.color_u);
            draw_rect(v, stride_v, sq.x / 2, sq.y / 2, sq.size / 2, sq.size / 2, sq.color_v);
            sq.x = (sq.x + fast_rand_u32() % 4) % width;
            sq.y = (sq.y + fast_rand_u32() % 4) % height;
        }

        draw_timestamp(y, stride_y, u, stride_u, 8, 8, 3);
    }
}

fn draw_rect(plane: &mut [u8], stride: u32, x: u32, y: u32, w: u32, h: u32, color: u8) {
    let stride = stride as usize;
    for row in y..y + h {
        let start = (row as usize) * stride + x as usize;
        let end = (start + w as usize).min(plane.len());
        if start < plane.len() {
            plane[start..end].fill(color);
        }
    }
}

// ── 5×7 bitmapped font for timestamp overlay ──

type Glyph = [u8; 7];

const FONT: &[u8] = &[
    // Upper 5 bits of each byte encode the glyph row (MSB = leftmost pixel).
    // 0
    0x70, 0x88, 0x88, 0x88, 0x88, 0x88, 0x70,
    // 1
    0x20, 0x60, 0x20, 0x20, 0x20, 0x20, 0x70,
    // 2
    0x70, 0x88, 0x08, 0x10, 0x20, 0x40, 0xF8,
    // 3
    0x70, 0x88, 0x08, 0x30, 0x08, 0x88, 0x70,
    // 4
    0x10, 0x30, 0x50, 0x90, 0xF8, 0x10, 0x10,
    // 5
    0xF8, 0x80, 0xF0, 0x08, 0x08, 0x88, 0x70,
    // 6
    0x70, 0x80, 0xF0, 0x88, 0x88, 0x88, 0x70,
    // 7
    0xF8, 0x08, 0x10, 0x20, 0x40, 0x40, 0x40,
    // 8
    0x70, 0x88, 0x88, 0x70, 0x88, 0x88, 0x70,
    // 9
    0x70, 0x88, 0x88, 0x78, 0x08, 0x08, 0x70,
    // :
    0x00, 0x00, 0x30, 0x30, 0x00, 0x30, 0x30,
    // .
    0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x30,
    // -
    0x00, 0x00, 0x00, 0xF8, 0x00, 0x00, 0x00,
    // ' '
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn glyph_index(ch: u8) -> usize {
    match ch {
        b'0'..=b'9' => (ch - b'0') as usize,
        b':' => 10,
        b'.' => 11,
        b'-' => 12,
        _ => 13, // space / unknown
    }
}

fn draw_glyph(plane: &mut [u8], stride_y: u32, x: u32, y: u32, glyph: &Glyph, scale: u32) {
    for row in 0..7u32 {
        let bits = glyph[row as usize];
        for col in 0..5u32 {
            if bits & (1u8 << (4 - col)) != 0 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x + col * scale + sx;
                        let py = y + row * scale + sy;
                        let idx = (py * stride_y + px) as usize;
                        if idx < plane.len() {
                            plane[idx] = 255; // white
                        }
                    }
                }
            }
        }
    }
}

fn draw_timestamp(y: &mut [u8], stride_y: u32, u: &mut [u8], stride_u: u32, x: u32, y_pos: u32, scale: u32) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let total_secs = now.as_secs();
    let millis = now.subsec_millis();

    let (year, month, day) = unix_to_date(total_secs);
    let sod = total_secs % 86400;
    let h = sod / 3600;
    let m = (sod % 3600) / 60;
    let s = sod % 60;

    let time_str = format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}", year, month, day, h, m, s, millis);

    let char_w = 6 * scale;
    let char_h = 7 * scale;
    let pad = 6;
    let text_w = time_str.len() as u32 * char_w;
    let text_h = char_h;

    // Semi-transparent black background: dark Y, neutral UV
    let bg_y = 16u8;
    let bg_u = 128u8;
    draw_rect(y, stride_y, x - pad, y_pos - pad, text_w + pad * 2, text_h + pad * 2, bg_y);
    draw_rect(u, stride_u, (x - pad) / 2, (y_pos - pad) / 2,
        (text_w + pad * 2) / 2, (text_h + pad * 2) / 2, bg_u);

    let mut cx = x;
    for ch in time_str.bytes() {
        let gi = glyph_index(ch);
        let glyph: &Glyph = &FONT[gi * 7..(gi + 1) * 7].try_into().unwrap();
        draw_glyph(y, stride_y, cx, y_pos, glyph, scale);
        cx += char_w;
    }
}

fn unix_to_date(unix_secs: u64) -> (u64, u64, u64) {
    let mut days = unix_secs / 86400;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }
    let month_days = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &md in month_days.iter() {
        if days < md { break; }
        days -= md;
        month += 1;
    }
    let day = days + 1;
    (year, month, day)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub struct VideoFrameGenerator {
    broadcaster: Arc<VideoBroadcaster<VideoFrame<Box<dyn VideoBuffer>>>>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
    // Lazy-start parameters (consumed on first start)
    start_config: Option<(u32, u32, u32, Option<Box<dyn FramePattern>>)>,
}

impl VideoFrameGenerator {
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        let pattern = SquarePattern::new(width, height, 10);
        Self::new_with_pattern(width, height, fps, Box::new(pattern))
    }

    pub fn new_with_pattern(width: u32, height: u32, fps: u32, pattern: Box<dyn FramePattern>) -> Self {
        let broadcaster = Arc::new(VideoBroadcaster::new());
        let running = Arc::new(AtomicBool::new(false));
        Self {
            broadcaster,
            running,
            thread_handle: None,
            start_config: Some((width, height, fps, Some(pattern))),
        }
    }

    pub fn start(&mut self) {
        if self.thread_handle.is_some() { return; }
        let Some((width, height, fps, pattern_opt)) = self.start_config.take() else { return; };
        let mut pattern = pattern_opt.unwrap_or_else(|| Box::new(SquarePattern::new(width, height, 10)));
        let rt = self.running.clone();
        let bc = self.broadcaster.clone();
        let frame_interval = Duration::from_micros((1_000_000 / fps as u64).max(1));
        rt.store(true, Ordering::Relaxed);

        let handle = thread::spawn(move || {
            while rt.load(Ordering::Relaxed) {
                let start = std::time::Instant::now();
                let mut buf = I420Buffer::new(width, height);
                pattern.draw(
                    &mut buf.data_y, &mut buf.data_u, &mut buf.data_v,
                    buf.stride_y, buf.stride_u, buf.stride_v,
                );
                let frame = VideoFrame::new(Box::new(buf) as Box<dyn VideoBuffer>);
                bc.on_frame(&frame);
                let elapsed = start.elapsed();
                if elapsed < frame_interval {
                    thread::sleep(frame_interval - elapsed);
                }
            }
        });
        self.thread_handle = Some(handle);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn sink_count(&self) -> usize {
        self.broadcaster.sink_count()
    }
}

impl VideoSource<VideoFrame<Box<dyn VideoBuffer>>> for VideoFrameGenerator {
    fn add_or_update_sink(&self, sink: Box<dyn VideoSink<VideoFrame<Box<dyn VideoBuffer>>>>, wants: VideoSinkWants) {
        self.broadcaster.add_or_update_sink(sink, wants);
    }

    fn remove_sink(&self, sink: &dyn VideoSink<VideoFrame<Box<dyn VideoBuffer>>>) {
        self.broadcaster.remove_sink(sink);
    }
}

impl Drop for VideoFrameGenerator {
    fn drop(&mut self) {
        self.stop();
    }
}
