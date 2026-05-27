use eframe::egui;

/// Fit an image into a container rect while preserving aspect ratio (centered).
pub fn fit_rect(img_size: [usize; 2], container: egui::Rect) -> egui::Rect {
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

/// Smoothstep easing: 3t² - 2t³
pub fn ease_in_out(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}
