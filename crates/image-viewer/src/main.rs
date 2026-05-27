mod app;
mod layout;

use eframe::egui;

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
        Box::new(|_cc| Ok(Box::<app::ImageViewer>::default())),
    )
}
