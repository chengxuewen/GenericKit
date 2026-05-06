use std::sync::Mutex;

/// Sink preferences for video adaptation.
#[derive(Debug, Clone)]
pub struct VideoSinkWants {
    pub rotation_applied: bool,
    pub max_pixel_count: u32,
    pub max_framerate_fps: u32,
    pub resolution_alignment: u32,
    pub is_active: bool,
}

impl Default for VideoSinkWants {
    fn default() -> Self {
        Self {
            rotation_applied: false,
            max_pixel_count: 0,
            max_framerate_fps: 0,
            resolution_alignment: 1,
            is_active: false,
        }
    }
}

pub trait VideoSink<F>: Send {
    fn on_frame(&self, frame: &F);
    fn on_discarded_frame(&self) {}
}

pub trait VideoSource<F>: Send {
    fn add_or_update_sink(&self, sink: Box<dyn VideoSink<F>>, wants: VideoSinkWants);
    fn remove_sink(&self, sink: &dyn VideoSink<F>);
}

pub trait AudioSink: Send {
    fn on_data(&self, samples: &[i16], sample_rate: u32, channels: u32);
}

pub trait AudioSource: Send {
    fn add_sink(&self, sink: Box<dyn AudioSink>);
    fn remove_sink(&self, sink: &dyn AudioSink);
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u32;
}

pub struct VideoBroadcaster<F> {
    pairs: Mutex<Vec<(Box<dyn VideoSink<F>>, VideoSinkWants)>>,
}

impl<F> VideoBroadcaster<F> {
    pub fn new() -> Self {
        Self {
            pairs: Mutex::new(Vec::new()),
        }
    }

    pub fn wants(&self) -> VideoSinkWants {
        let pairs = self.pairs.lock().unwrap();
        aggregate_wants(pairs.iter().map(|(_, w)| w))
    }

    pub fn sink_count(&self) -> usize {
        self.pairs.lock().unwrap().len()
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    let mut x = a;
    let mut y = b;
    while y != 0 {
        let t = y;
        y = x % y;
        x = t;
    }
    x
}

fn lcm(a: u32, b: u32) -> u32 {
    if a == 0 || b == 0 {
        return 0;
    }
    a / gcd(a, b) * b
}

pub fn aggregate_wants<'a>(wants: impl Iterator<Item = &'a VideoSinkWants>) -> VideoSinkWants {
    let mut result = VideoSinkWants::default();
    for w in wants {
        result.rotation_applied |= w.rotation_applied;
        result.is_active |= w.is_active;
        if w.max_pixel_count > 0 {
            if result.max_pixel_count == 0 {
                result.max_pixel_count = w.max_pixel_count;
            } else {
                result.max_pixel_count = result.max_pixel_count.min(w.max_pixel_count);
            }
        }
        if w.max_framerate_fps > 0 {
            if result.max_framerate_fps == 0 {
                result.max_framerate_fps = w.max_framerate_fps;
            } else {
                result.max_framerate_fps = result.max_framerate_fps.min(w.max_framerate_fps);
            }
        }
        if w.resolution_alignment > 1 {
            result.resolution_alignment = lcm(result.resolution_alignment, w.resolution_alignment);
        }
    }
    result
}

impl<F: Send + 'static> VideoSink<F> for VideoBroadcaster<F> {
    fn on_frame(&self, frame: &F) {
        let pairs = self.pairs.lock().unwrap();
        for (sink, wants) in pairs.iter() {
            if wants.is_active {
                sink.on_frame(frame);
            }
        }
    }
}

impl<F: Send + 'static> VideoSource<F> for VideoBroadcaster<F> {
    fn add_or_update_sink(&self, sink: Box<dyn VideoSink<F>>, wants: VideoSinkWants) {
        let mut pairs = self.pairs.lock().unwrap();
        pairs.push((sink, wants));
    }

    fn remove_sink(&self, sink: &dyn VideoSink<F>) {
        let mut pairs = self.pairs.lock().unwrap();
        pairs.retain(|(s, _)| {
            !std::ptr::eq(
                s.as_ref() as *const (dyn VideoSink<F>) as *const (),
                sink as *const (dyn VideoSink<F>) as *const (),
            )
        });
    }
}

/// Default silence audio source.
pub struct DefaultAudioSource {
    sample_rate: u32,
    channels: u32,
    sinks: Mutex<Vec<Box<dyn AudioSink>>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl DefaultAudioSource {
    pub fn new(sample_rate: u32, channels: u32) -> Self {
        Self {
            sample_rate,
            channels,
            sinks: Mutex::new(Vec::new()),
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    pub fn start(&mut self) {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        let sr = self.sample_rate;
        let ch = self.channels;
        let running = self.running.clone();
        let mut sinks = Mutex::new(Vec::new());
        // Move existing sinks
        std::mem::swap(&mut sinks, &mut self.sinks);

        let handle = std::thread::spawn(move || {
            let frame_samples = (sr / 50) as usize; // 20ms frames
            let silence = vec![0i16; frame_samples * ch as usize];
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let sinks = sinks.lock().unwrap();
                for sink in sinks.iter() {
                    sink.on_data(&silence, sr, ch);
                }
                drop(sinks);
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        });
        self.thread_handle = Some(handle);
    }

    pub fn stop(&mut self) {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl AudioSource for DefaultAudioSource {
    fn add_sink(&self, sink: Box<dyn AudioSink>) {
        self.sinks.lock().unwrap().push(sink);
    }

    fn remove_sink(&self, sink: &dyn AudioSink) {
        self.sinks.lock().unwrap().retain(|s| {
            !std::ptr::eq(
                s.as_ref() as *const (dyn AudioSink) as *const (),
                sink as *const (dyn AudioSink) as *const (),
            )
        });
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u32 {
        self.channels
    }
}

impl Drop for DefaultAudioSource {
    fn drop(&mut self) {
        self.stop();
    }
}
