use std::collections::VecDeque;
use crate::video::source_sink::VideoSinkWants;

pub struct VideoAdapter {
    target_pixels: u32,
    max_fps: f32,
    frame_timestamps: VecDeque<i64>,
}

impl VideoAdapter {
    pub fn new() -> Self {
        Self {
            target_pixels: 0,
            max_fps: 0.0,
            frame_timestamps: VecDeque::new(),
        }
    }

    pub fn on_sink_wants(&mut self, wants: &VideoSinkWants) {
        if wants.max_pixel_count > 0 {
            if self.target_pixels == 0 {
                self.target_pixels = wants.max_pixel_count;
            } else {
                self.target_pixels = self.target_pixels.min(wants.max_pixel_count);
            }
        }
        if wants.max_framerate_fps > 0 {
            self.max_fps = wants.max_framerate_fps as f32;
        }
    }

    pub fn adapt_frame(&mut self, in_w: u32, in_h: u32, timestamp_us: i64)
        -> Option<(u32, u32, u32, u32, u32, u32)>
    {
        if self.max_fps > 0.0 {
            self.frame_timestamps.push_back(timestamp_us);
            let window_us = (1_000_000.0 / self.max_fps) as i64;
            while self.frame_timestamps.len() > 1 {
                let oldest = *self.frame_timestamps.front().unwrap();
                if timestamp_us - oldest > window_us {
                    self.frame_timestamps.pop_front();
                } else {
                    break;
                }
            }
            if self.frame_timestamps.len() > 1 {
                return None;
            }
        }

        if self.target_pixels > 0 {
            let in_pixels = in_w * in_h;
            if in_pixels <= self.target_pixels {
                return Some((0, 0, in_w, in_h, in_w, in_h));
            }
            let scale = (self.target_pixels as f64 / in_pixels as f64).sqrt();
            let out_w = ((in_w as f64 * scale) as u32).max(2);
            let out_h = ((in_h as f64 * scale) as u32).max(2);
            return Some((0, 0, in_w, in_h, out_w, out_h));
        }

        Some((0, 0, in_w, in_h, in_w, in_h))
    }
}
