use eframe::egui;

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

#[derive(Default)]
struct ImageViewer {}

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("Hello from image-viewer!");
            });
        });
    }
}
