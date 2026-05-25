# Image Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现一个 Windows 图片查看器，支持拖入两张图片，通过空格键淡入淡出切换。

**Architecture:** 单 crate (`image-viewer`)，单文件 `main.rs`，基于 eframe/egui 的 App trait 实现。状态机管理 Idle/Transitioning 两种状态，通过 painter 自定义绘制实现 crossfade。

**Tech Stack:** Rust edition 2024, eframe 0.31 (egui + wgpu), image 0.25

---

### Task 1: 项目结构搭建

**Files:**
- Modify: `Cargo.toml` (root)
- Create: `crates/image-viewer/Cargo.toml`
- Create: `crates/image-viewer/src/main.rs`
- Delete: `src/main.rs` (remove old package default)

- [ ] **Step 1: 将根 Cargo.toml 改为 workspace**

将 `Cargo.toml` 内容替换为：

```toml
[workspace]
members = ["crates/*"]
resolver = "2"
```

- [ ] **Step 2: 创建 image-viewer crate 的 Cargo.toml**

新建 `crates/image-viewer/Cargo.toml`：

```toml
[package]
name = "image-viewer"
version = "0.1.0"
edition = "2024"

[dependencies]
eframe = "0.31"
image = "0.25"
```

- [ ] **Step 3: 创建 main.rs 骨架**

新建 `crates/image-viewer/src/main.rs`：

```rust
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
```

- [ ] **Step 4: 删除旧 src/main.rs**

```bash
rm src/main.rs
```

`src/` 目录也会被删除（如果为空）。

- [ ] **Step 5: 构建验证**

```bash
cargo build
```

Expected: 编译成功，无错误。

- [ ] **Step 6: 提交**

```bash
git add Cargo.toml crates/ src/
git commit -m "feat: set up workspace with image-viewer crate skeleton"
```

---

### Task 2: 核心类型与图片加载

**Files:**
- Modify: `crates/image-viewer/src/main.rs`

- [ ] **Step 1: 添加 LoadedImage 结构体和图片加载方法**

将 `crates/image-viewer/src/main.rs` 替换为：

```rust
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
```

- [ ] **Step 2: 构建验证**

```bash
cargo build
```

Expected: 编译成功。

- [ ] **Step 3: 提交**

```bash
git add crates/
git commit -m "feat: add LoadedImage type and image loading from file path"
```

---

### Task 3: 拖放处理与图片渲染

**Files:**
- Modify: `crates/image-viewer/src/main.rs`

- [ ] **Step 1: 添加 aspect-ratio fit 工具函数和 hint 文本**

将 `crates/image-viewer/src/main.rs` 完整替换为包含拖放和渲染的版本：

```rust
use std::path::PathBuf;

use eframe::egui::{self, Color32, ColorImage, TextureOptions};
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

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Read all input at once to avoid borrow conflicts with ctx.load_texture
        let dropped_files: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path.clone())
                .collect()
        });

        // Process dropped files
        for path in &dropped_files {
            self.load_image(path, ctx);
        }

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

                // Draw img1
                if let Some(img) = &self.img1 {
                    let rect = fit_rect(img.size, available);
                    painter.image(
                        img.texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
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
```

- [ ] **Step 2: 构建验证**

```bash
cargo build
```

Expected: 编译成功。

- [ ] **Step 3: 提交**

```bash
git add crates/
git commit -m "feat: add drag-drop image loading and aspect-ratio rendering"
```

---

### Task 4: 空格键淡入淡出切换

**Files:**
- Modify: `crates/image-viewer/src/main.rs`

- [ ] **Step 1: 添加过渡动画状态和 easing 函数**

将 `crates/image-viewer/src/main.rs` 完整替换为最终版本：

```rust
use std::path::PathBuf;

use eframe::egui::{self, Color32, ColorImage, TextureOptions};
use image::GenericImageView;

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
                .map(|f| f.path.clone())
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
```

- [ ] **Step 2: 构建验证**

```bash
cargo build
```

Expected: 编译成功，无警告。

- [ ] **Step 3: 提交**

```bash
git add crates/
git commit -m "feat: add space-key crossfade transition between two images"
```

---

### Task 5: 运行验证

- [ ] **Step 1: 运行应用**

```bash
cargo run -p image-viewer
```

Expected: 窗口打开，显示 "Drag images here" 提示文字。

- [ ] **Step 2: 手动测试**

1. 从文件管理器拖入一张图片 → 确认显示该图片，状态栏显示 "Image 1 loaded"
2. 拖入第二张图片 → 状态栏显示 "Image 2 loaded"
3. 按住空格键 → 图片平滑过渡到第二张 (crossfade, ~0.3s)
4. 松开空格键 → 图片平滑过渡回第一张
5. 拖入第三张图片 → 覆盖第二张（img2 被替换）

- [ ] **Step 3: 提交（如有修改）**

如有修改，提交并推送。
