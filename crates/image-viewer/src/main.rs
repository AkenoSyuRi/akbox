use std::path::PathBuf;

use eframe::egui::{self, ColorImage, TextureOptions};
use image::GenericImageView;

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

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(&self.status_text);
            });
        });
    }
}
