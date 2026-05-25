use std::path::PathBuf;

use eframe::egui::{self, Color32, ColorImage, TextureOptions};

const TRANSITION_DURATION: f32 = 0.3;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Akbox Image Viewer",
        options,
        Box::new(|_cc| Ok(Box::<ImageViewer>::default())),
    )
}

struct LoadedImage {
    texture: egui::TextureHandle,
    size: [usize; 2],
}

#[derive(Default)]
struct ImageViewer {
    img1: Option<LoadedImage>,
    img2: Option<LoadedImage>,
    transition: f32, // 0.0 = img1 fully visible, 1.0 = img2 fully visible
    target: f32,     // target blend value (0.0 or 1.0)
    status_text: String,
}

impl ImageViewer {
    fn load_image(&mut self, path: &PathBuf, ctx: &egui::Context) {
        let img = match image::open(path) {
            Ok(img) => img,
            Err(e) => {
                self.status_text = format!("Failed to load: {}", e);
                return;
            }
        };
        let size = [img.width() as usize, img.height() as usize];
        let rgba = img.to_rgba8();
        let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let texture = ctx.load_texture(name, color_image, TextureOptions::default());

        let loaded = LoadedImage { texture, size };

        if self.img1.is_none() {
            self.status_text = format!(
                "Image 1 loaded: {}",
                path.file_name().unwrap().to_string_lossy()
            );
            self.img1 = Some(loaded);
        } else {
            self.status_text = format!(
                "Image 2 loaded: {}",
                path.file_name().unwrap().to_string_lossy()
            );
            self.img2 = Some(loaded);
        }
    }
}

fn fit_rect(img_size: [usize; 2], container: egui::Rect) -> egui::Rect {
    let img_aspect = img_size[0] as f32 / img_size[1] as f32;
    let container_aspect = container.width() / container.height().max(0.001);

    let (w, h) = if img_aspect > container_aspect {
        let w = container.width();
        (w, w / img_aspect)
    } else {
        let h = container.height();
        (h * img_aspect, h)
    };

    let min = egui::pos2(
        container.center().x - w / 2.0,
        container.center().y - h / 2.0,
    );
    egui::Rect::from_min_size(min, egui::vec2(w, h))
}

/// Smoothstep easing: ease-in-out
fn ease_in_out(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Read all input at once to avoid borrow conflicts
        let (dropped_files, space_pressed, space_released, dt) = ctx.input(|i| {
            let files: Vec<PathBuf> = i
                .raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect();
            (
                files,
                i.key_down(egui::Key::Space),
                i.key_released(egui::Key::Space),
                i.stable_dt.min(0.1),
            )
        });

        // Process dropped files
        for path in &dropped_files {
            self.load_image(path, ctx);
        }

        // Handle space key (only when both images are loaded)
        if self.img2.is_some() {
            if space_pressed {
                self.target = 1.0;
            }
            if space_released {
                self.target = 0.0;
            }
        }

        // Update transition animation
        let speed = 1.0 / TRANSITION_DURATION;
        let diff = self.target - self.transition;
        if diff.abs() > 0.001 {
            let step = speed * dt;
            if diff > 0.0 {
                self.transition = (self.transition + step).min(self.target);
            } else {
                self.transition = (self.transition - step).max(self.target);
            }
            ctx.request_repaint(); // Keep animating
        }

        let display_t = ease_in_out(self.transition);

        // Render
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_rect_before_wrap();

            if self.img1.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Drag images here")
                            .size(24.0)
                            .color(Color32::GRAY),
                    );
                });
            } else {
                let painter = ui.painter();

                // Draw img1 (alpha decreases as display_t increases)
                if let Some(img) = &self.img1 {
                    let rect = fit_rect(img.size, available);
                    let alpha = ((1.0 - display_t) * 255.0) as u8;
                    painter.image(
                        img.texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::from_rgba_premultiplied(255, 255, 255, alpha),
                    );
                }

                // Draw img2 (alpha increases as display_t increases)
                if let Some(img) = &self.img2 {
                    let rect = fit_rect(img.size, available);
                    let alpha = (display_t * 255.0) as u8;
                    painter.image(
                        img.texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::from_rgba_premultiplied(255, 255, 255, alpha),
                    );
                }
            }
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label(&self.status_text);
        });
    }
}
