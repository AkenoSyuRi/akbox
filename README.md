# Akbox Image Viewer

A minimal image viewer built with Rust and [egui](https://github.com/emilk/egui) (eframe).

## Features

- **Drag & drop** images into the window to load (up to 9)
- **Thumbnail panel** on the right — current image highlighted with a red border and always centered
- **Click** a thumbnail to switch; click the **✕ icon** in its top-right corner to remove
- **Mouse wheel** anywhere in the window to switch images (1 tick = 1 image, no wrap-around)
- **Number keys** `1`–`9` to jump to a specific image
- **Space bar** — hold to crossfade between current and next image

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `eframe` | 0.31 | Native window + egui rendering |
| `image` | 0.25 | Image decoding (PNG, JPEG, etc.) |
| `ctrlc` | 3.4 | Ctrl-C signal handler |

## Build & Run

```sh
cargo run --manifest-path crates/image-viewer/Cargo.toml
```

Check only:

```sh
cargo check --manifest-path crates/image-viewer/Cargo.toml
```

## Project Layout

```
crates/image-viewer/src/
├── main.rs     — Entry point, window setup
├── app.rs      — ImageViewer struct, image loading, UI rendering
└── layout.rs   — Aspect-ratio fitting and animation easing helpers
```
