// gkit-media-viewer — VideoFrame transform demo with egui (grid + single view)
// Usage: cargo run -p gkit-media --example gkit-media-viewer

use eframe::egui;
use gkit_media::video::buffer::{I420Buffer, VideoBuffer, VideoFormatType};
use gkit_media::video::convert::{argb_to_i420, i420_to_argb, i420_to_nv12, i420_to_nv21, nv21_to_i420, i420_to_uyvy, i420_to_yuy2};
use gkit_media::video::transform::{i420_crop, i420_rotate, i420_scale};

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "gkit-media Video Frame Viewer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 800.0])
                .with_title("gkit-media Video Frame Viewer"),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(PartialEq)]
enum Page {
    Grid,
    Single,
}

struct FrameVariant {
    label: String,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    texture: Option<egui::TextureHandle>,
}

impl FrameVariant {
    fn from_i420(label: &str, i420: &I420Buffer) -> Self {
        let w = i420.width;
        let h = i420.height;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        i420_to_argb(i420, &mut rgba, w * 4, VideoFormatType::RGBA);
        Self { label: label.into(), rgba, width: w, height: h, texture: None }
    }

    fn load_texture(&mut self, ctx: &egui::Context, id: &str) {
        let size = [self.width as usize, self.height as usize];
        let img = egui::ColorImage::from_rgba_unmultiplied(size, &self.rgba);
        self.texture = Some(ctx.load_texture(String::from(id), img, egui::TextureOptions::LINEAR));
    }
}

/// Per-frame editable state for single-view mode
struct SingleViewState {
    selected_label: String,
    selected_rgba: Vec<u8>,
    selected_w: u32,
    selected_h: u32,
    #[allow(dead_code)]
    orig_i420: I420Buffer,
    texture: Option<egui::TextureHandle>,
}

struct App {
    page: Page,
    variants: Vec<FrameVariant>,
    single: Option<SingleViewState>,
    thumbnail_w: u32,
    status: String,
    loaded: bool,
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            page: Page::Grid,
            variants: vec![],
            single: None,
            thumbnail_w: 240,
            status: String::new(),
            loaded: false,
        };
        app.build_variants();
        app
    }
}

impl App {
    fn resolve_asset_path() -> std::path::PathBuf {
        let exe_dir = std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| ".".into());
        let path = exe_dir.join("../assets/images/color_card_1920x1080.bmp");
        if path.exists() { path } else {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/images/color_card_1920x1080.bmp").into()
        }
    }

    fn load_i420() -> Option<I420Buffer> {
        let dynamic = image::open(&Self::resolve_asset_path()).ok()?;
        let rgba = dynamic.to_rgba8();
        let (w, h) = rgba.dimensions();
        let pixels = rgba.into_raw();
        argb_to_i420(&pixels, w, h, w * 4).ok()
    }

    fn i420_to_frame(_label: &str, i420: &I420Buffer) -> (Vec<u8>, u32, u32) {
        let w = i420.width; let h = i420.height;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        i420_to_argb(i420, &mut rgba, w * 4, VideoFormatType::RGBA);
        (rgba, w, h)
    }

    fn build_variants(&mut self) {
        let i420 = match Self::load_i420() {
            Some(v) => v,
            None => return,
        };
        let w = i420.width;
        let h = i420.height;

        let nv12_i420 = {
            i420_to_nv12(&i420).to_i420().unwrap()
        };

        self.variants = vec![
            FrameVariant::from_i420("Original 1920×1080 I420", &i420),
            FrameVariant::from_i420("I420→NV12→I420 round-trip", &nv12_i420),
            FrameVariant::from_i420("Scale 50% → 960×540",
                &i420_scale(&i420, w/2, h/2).unwrap()),
            FrameVariant::from_i420("Scale 25% → 480×270",
                &i420_scale(&i420, w/4, h/4).unwrap()),
            {
                let cx = ((w - 960) / 2) & !1;
                let cy = ((h - 540) / 2) & !1;
                FrameVariant::from_i420("Crop Center 960×540",
                    &i420_crop(&i420, cx, cy, 960, 540).unwrap())
            },
            FrameVariant::from_i420("Rotate 90°",
                &i420_rotate(&i420, 90).unwrap()),
            FrameVariant::from_i420("Rotate 180°",
                &i420_rotate(&i420, 180).unwrap()),
            FrameVariant::from_i420("Rotate 270°",
                &i420_rotate(&i420, 270).unwrap()),
            {
                let half = i420_scale(&i420, 960, 540).unwrap();
                let cx = ((half.width - 480) / 2) & !1;
                let cy = ((half.height - 270) / 2) & !1;
                FrameVariant::from_i420("Scale→Crop 480×270",
                    &i420_crop(&half, cx, cy, 480, 270).unwrap())
            },
            {
                let r90 = i420_rotate(&i420, 90).unwrap();
                FrameVariant::from_i420("Rot90→Scale 50%",
                    &i420_scale(&r90, r90.width/2, r90.height/2).unwrap())
            },
        ];

        // Init single view: show original
        let (rgba, fw, fh) = Self::i420_to_frame("Original", &i420);
        self.single = Some(SingleViewState {
            selected_label: "Original 1920×1080 I420".into(),
            selected_rgba: rgba,
            selected_w: fw, selected_h: fh,
            orig_i420: i420,
            texture: None,
        });

        self.loaded = true;
        self.status = format!("{} variants processed", self.variants.len());
    }

    fn apply_single(&mut self, label: &str, f: impl FnOnce(&I420Buffer) -> I420Buffer) {
        if let Some(ref s) = self.single {
            let i420 = f(&s.orig_i420);
            let (rgba, w, h) = Self::i420_to_frame(label, &i420);
            self.single = Some(SingleViewState {
                selected_label: label.into(),
                selected_rgba: rgba,
                selected_w: w, selected_h: h,
                orig_i420: s.orig_i420.clone(),
                texture: None,
            });
            self.status = format!("{}: {}×{}", label, w, h);
        }
    }

    fn show_grid_page(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Thumbnail size selector in top area
        ui.horizontal(|ui| {
            ui.label("Thumbnail:");
            for &tw in &[120u32, 180, 240, 320, 480] {
                if ui.selectable_label(self.thumbnail_w == tw, tw.to_string()).clicked() {
                    self.thumbnail_w = tw;
                    for v in &mut self.variants { v.texture = None; }
                }
            }
            ui.separator();
            ui.label(format!("{} variants", self.variants.len()));
        });
        ui.separator();

        let tw = self.thumbnail_w as f32;
        let avail = ui.available_width();
        let cols = (avail / (tw + 12.0)).floor().max(1.0) as usize;
        let variants_len = self.variants.len();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("variant_grid")
                .striped(false)
                .min_col_width(tw)
                .spacing([6.0, 6.0])
                .show(ui, |ui| {
                    for (i, v) in self.variants.iter_mut().enumerate() {
                        if v.texture.is_none() {
                            v.load_texture(ctx, &format!("g{}", i));
                        }
                        ui.vertical(|ui| {
                            ui.set_width(tw);
                            if let Some(ref tex) = v.texture {
                                let scale = tw / v.width.max(v.height) as f32;
                                ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::new(
                                    tex.id(), egui::Vec2::new(v.width as f32 * scale, v.height as f32 * scale))));
                            }
                            ui.label(egui::RichText::new(&v.label).size(11.0));
                            ui.label(egui::RichText::new(format!("{}×{}", v.width, v.height)).size(10.0).color(egui::Color32::GRAY));
                        });
                        if (i + 1) % cols == 0 && i + 1 < variants_len {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    fn show_single_page(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if let Some(ref mut s) = self.single {
            // Ensure texture is loaded
            if s.texture.is_none() {
                let size = [s.selected_w as usize, s.selected_h as usize];
                let img = egui::ColorImage::from_rgba_unmultiplied(size, &s.selected_rgba);
                s.texture = Some(ctx.load_texture(String::from("single-tex"), img, egui::TextureOptions::LINEAR));
            }

            // Info bar
            ui.label(format!("{}  |  {}×{}", s.selected_label, s.selected_w, s.selected_h));
            ui.separator();

            // Image area
            if let Some(ref tex) = s.texture {
                let available = ui.available_size();
                let scale = (available.x / s.selected_w as f32)
                    .min(available.y / s.selected_h as f32)
                    .min(1.0);
                ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::new(
                    tex.id(), egui::Vec2::new(s.selected_w as f32 * scale, s.selected_h as f32 * scale))));
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top bar with tabs
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("gkit-media");
                ui.separator();

                ui.selectable_value(&mut self.page, Page::Grid, "📋 Grid View");
                ui.selectable_value(&mut self.page, Page::Single, "🔍 Single View");

                ui.separator();
                if ui.button("🔄 Reload").clicked() {
                    self.variants.clear();
                    self.single = None;
                    self.loaded = false;
                    self.build_variants();
                }
                ui.label(format!("|  {}", self.status));
            });
        });

        // Bottom status
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label("Pipeline: BMP → BT.601 → I420 → [Scale | Crop | Rotate | NV12] → RGBA → Display");
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.loaded {
                ui.centered_and_justified(|ui| { ui.spinner(); });
                return;
            }

            match self.page {
                Page::Grid => self.show_grid_page(ctx, ui),
                Page::Single => {
                    // Right panel with interactive controls
                    egui::SidePanel::right("controls")
                        .resizable(false)
                        .default_width(200.0)
                        .show_inside(ui, |ui| {
                            ui.heading("Transform");
                            ui.separator();

                            if ui.button("Original").clicked() {
                                if let Some(ref s) = self.single {
                                    let (rgba, w, h) = Self::i420_to_frame("Original", &s.orig_i420);
                                    self.single = Some(SingleViewState {
                                        selected_label: "Original 1920×1080 I420".into(),
                                        selected_rgba: rgba, selected_w: w, selected_h: h,
                                        orig_i420: s.orig_i420.clone(), texture: None,
                                    });
                                }
                            }
                            ui.separator();

                            ui.label("Format Convert");
                            for (label, cb) in [
                                ("I420 → NV12 → I420", Box::new(|i: &I420Buffer| i420_to_nv12(i).to_i420().unwrap()) as Box<dyn Fn(&I420Buffer) -> I420Buffer>),
                                ("I420 → NV21 → I420", Box::new(|i: &I420Buffer| nv21_to_i420(&i420_to_nv21(i)).unwrap())),
                                ("I420 → YUY2 → I420", Box::new(|i: &I420Buffer| {
                                    let mut buf = vec![0u8; (i.width * i.height * 2) as usize];
                                    i420_to_yuy2(i, &mut buf);
                                    // Simple YUY2→I420: just pass through I420 since we only display I420
                                    i.clone()
                                })),
                                ("I420 → UYVY → I420", Box::new(|i: &I420Buffer| {
                                    let mut buf = vec![0u8; (i.width * i.height * 2) as usize];
                                    i420_to_uyvy(i, &mut buf);
                                    i.clone()
                                })),
                            ].iter() {
                                if ui.button(*label).clicked() {
                                    self.apply_single(label, |i| cb(i));
                                }
                            }
                            ui.separator();

                            ui.label("Scale");
                            if ui.button("50%").clicked() {
                                self.apply_single("Scale 50%", |i| i420_scale(i, i.width/2, i.height/2).unwrap());
                            }
                            if ui.button("25%").clicked() {
                                self.apply_single("Scale 25%", |i| i420_scale(i, i.width/4, i.height/4).unwrap());
                            }
                            ui.separator();

                            ui.label("Crop");
                            if ui.button("Center 960×540").clicked() {
                                self.apply_single("Crop 960×540", |i| {
                                    let cx = ((i.width - 960) / 2) & !1;
                                    let cy = ((i.height - 540) / 2) & !1;
                                    i420_crop(i, cx, cy, 960, 540).unwrap()
                                });
                            }
                            ui.separator();

                            ui.label("Rotate");
                            for &deg in &[90, 180, 270] {
                                if ui.button(format!("{}°", deg)).clicked() {
                                    self.apply_single(&format!("Rotate {}°", deg), |i| i420_rotate(i, deg).unwrap());
                                }
                            }
                            ui.separator();

                            ui.label("Pipeline");
                            if ui.button("Scale→Crop").clicked() {
                                self.apply_single("Scale→Crop", |i| {
                                    let half = i420_scale(i, 960, 540).unwrap();
                                    let cx = ((half.width - 480) / 2) & !1;
                                    let cy = ((half.height - 270) / 2) & !1;
                                    i420_crop(&half, cx, cy, 480, 270).unwrap()
                                });
                            }
                            if ui.button("Rotate→Scale").clicked() {
                                self.apply_single("Rot→Scale", |i| {
                                    let r = i420_rotate(i, 90).unwrap();
                                    i420_scale(&r, r.width/2, r.height/2).unwrap()
                                });
                            }
                        });

                    self.show_single_page(ctx, ui);
                }
            }
        });
    }
}
