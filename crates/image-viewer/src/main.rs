use std::path::PathBuf;

use eframe::egui::{self, Color32, ColorImage, TextureOptions};

const TRANSITION_DURATION: f32 = 0.1;
const MAX_IMAGES: usize = 9;
const THUMB_PANEL_WIDTH: f32 = 140.0;

fn main() -> Result<(), eframe::Error> {
    ctrlc::set_handler(|| {
        std::process::exit(0);
    })
    .expect("failed to set Ctrl-C handler");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_drag_and_drop(true),
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
    images: Vec<LoadedImage>,
    current_index: usize,
    transition: f32,
    target: f32,
    status_text: String,
}

impl ImageViewer {
    fn load_image(&mut self, path: &PathBuf, ctx: &egui::Context) {
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_else(|| "unknown".into());

        if self.images.len() >= MAX_IMAGES {
            self.status_text = format!("Max {} images", MAX_IMAGES);
            return;
        }

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

        self.images.push(LoadedImage { texture, size });
        self.current_index = self.images.len() - 1;
        self.status_text = format!("Image {} loaded: {}", self.current_index + 1, file_name);
    }

    fn remove_image(&mut self, index: usize) {
        if index < self.images.len() {
            self.images.remove(index);
            if self.images.is_empty() {
                self.current_index = 0;
            } else if self.current_index >= self.images.len() {
                self.current_index = self.images.len() - 1;
            }
            self.transition = 0.0;
            self.target = 0.0;
            self.status_text = format!("Image {} removed", index + 1);
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

fn ease_in_out(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Read all input once ──
        let (dropped_files, hovered_files, space_pressed, space_released, dt, clicked, hover_pos, scroll_delta, num_key) = ctx.input(|i| {
            let files: Vec<PathBuf> = i
                .raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect();

            let num_key = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3,
                           egui::Key::Num4, egui::Key::Num5, egui::Key::Num6,
                           egui::Key::Num7, egui::Key::Num8, egui::Key::Num9]
                .iter()
                .position(|k| i.key_pressed(*k));

            (
                files,
                !i.raw.hovered_files.is_empty(),
                i.key_down(egui::Key::Space),
                i.key_released(egui::Key::Space),
                i.stable_dt.min(0.1),
                i.pointer.button_clicked(egui::PointerButton::Primary),
                i.pointer.hover_pos(),
                i.smooth_scroll_delta.y,
                num_key,
            )
        });

        // ── Process dropped files ──
        for path in &dropped_files {
            self.load_image(path, ctx);
        }

        // ── Number keys ──
        if let Some(n) = num_key
            && n < self.images.len() {
                self.current_index = n;
                self.transition = 0.0;
                self.target = 0.0;
            }

        // ── Space key ──
        if self.images.len() >= 2 {
            if space_pressed {
                self.target = 1.0;
            }
            if space_released {
                self.target = 0.0;
            }
        }

        // ── Transition animation ──
        let speed = 1.0 / TRANSITION_DURATION;
        let diff = self.target - self.transition;
        if diff.abs() > 0.001 {
            let step = speed * dt;
            if diff > 0.0 {
                self.transition = (self.transition + step).min(self.target);
            } else {
                self.transition = (self.transition - step).max(self.target);
            }
            ctx.request_repaint();
        }
        let display_t = ease_in_out(self.transition);

        // ── Thumbnail interaction state (collected during rendering) ──
        let mut thumbnail_actions: Vec<ThumbnailAction> = Vec::new();

        // ── Right thumbnail panel ──
        egui::SidePanel::right("thumbnails")
            .min_width(THUMB_PANEL_WIDTH)
            .resizable(false)
            .show(ctx, |ui| {
                // Mouse wheel over panel: change current_index
                if scroll_delta != 0.0
                    && let Some(pos) = hover_pos
                        && ui.max_rect().contains(pos) {
                            if scroll_delta > 0.0 {
                                let n = self.images.len();
                                if n > 0 {
                                    self.current_index = (self.current_index + n - 1) % n;
                                    self.transition = 0.0;
                                    self.target = 0.0;
                                }
                            } else {
                                let n = self.images.len();
                                if n > 0 {
                                    self.current_index = (self.current_index + 1) % n;
                                    self.transition = 0.0;
                                    self.target = 0.0;
                                }
                            }
                        }

                if self.images.is_empty() {
                    ui.label("No images");
                    return;
                }

                let panel_rect = ui.max_rect();
                let thumb_padding = 6.0;
                let thumb_width = panel_rect.width() - thumb_padding * 2.0;
                let spacing = 6.0;
                let mut y = panel_rect.top() + 4.0;

                let painter = ui.painter();

                for i in 0..self.images.len() {
                    let img = &self.images[i];
                    let img_aspect = img.size[0] as f32 / img.size[1] as f32;
                    let thumb_height = thumb_width / img_aspect;
                    let item_height = thumb_height + spacing;

                    let thumb_rect = egui::Rect::from_min_size(
                        egui::pos2(panel_rect.left() + thumb_padding, y),
                        egui::vec2(thumb_width, thumb_height),
                    );

                    // Detect hover
                    let is_hovered = hover_pos.is_some_and(|p| thumb_rect.contains(p));

                    // Highlight current image
                    if i == self.current_index {
                        painter.rect_stroke(
                            thumb_rect.expand(2.0),
                            0.0,
                            egui::Stroke::new(2.0, Color32::LIGHT_BLUE),
                            egui::StrokeKind::Middle,
                        );
                    }

                    // Draw thumbnail
                    let fit = fit_rect(img.size, thumb_rect);
                    painter.image(
                        img.texture.id(),
                        fit,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );

                    // Number label (top-left shadowed)
                    let label_pos = egui::pos2(thumb_rect.left() + 2.0, thumb_rect.top() + 1.0);
                    painter.text(
                        label_pos,
                        egui::Align2::LEFT_TOP,
                        format!("{}", i + 1),
                        egui::FontId::proportional(16.0),
                        Color32::WHITE,
                    );

                    if is_hovered {
                        // Dark overlay + delete icon
                        painter.rect_filled(
                            thumb_rect,
                            0.0,
                            Color32::from_rgba_premultiplied(0, 0, 0, 160),
                        );
                        painter.text(
                            thumb_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "✕",
                            egui::FontId::proportional(28.0),
                            Color32::from_rgba_premultiplied(255, 80, 80, 255),
                        );

                        if clicked {
                            thumbnail_actions.push(ThumbnailAction::Remove(i));
                        }
                    } else if clicked {
                        thumbnail_actions.push(ThumbnailAction::Select(i));
                    }

                    y += item_height;
                }
            });

        // ── Process thumbnail actions (deferred) ──
        for action in thumbnail_actions {
            match action {
                ThumbnailAction::Select(i) => {
                    if i < self.images.len() {
                        self.current_index = i;
                        self.transition = 0.0;
                        self.target = 0.0;
                    }
                }
                ThumbnailAction::Remove(i) => {
                    self.remove_image(i);
                }
            }
        }

        // ── Main content ──
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_rect_before_wrap();

            if self.images.is_empty() {
                ui.centered_and_justified(|ui| {
                    if hovered_files {
                        ui.label(
                            egui::RichText::new("Drop to load image")
                                .size(24.0)
                                .color(Color32::LIGHT_BLUE),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("Drag images here")
                                .size(24.0)
                                .color(Color32::GRAY),
                        );
                    }
                });
            } else {
                let painter = ui.painter();

                // Base: current image at full opacity
                if let Some(img) = self.images.get(self.current_index) {
                    let rect = fit_rect(img.size, available);
                    painter.image(
                        img.texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                // Fade overlay: next image (space key)
                if self.images.len() >= 2 {
                    let next_index = (self.current_index + 1) % self.images.len();
                    if let Some(img) = self.images.get(next_index) {
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
            }
        });

        // ── Status bar ──
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label(&self.status_text);
        });
    }
}

enum ThumbnailAction {
    Select(usize),
    Remove(usize),
}
