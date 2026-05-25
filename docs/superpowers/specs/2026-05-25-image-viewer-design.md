# Image Viewer - Design Spec

## Overview

`image-viewer` 是 akbox 工具集的第一个 crate，一个 Windows 图片查看器。支持拖入两张图片，通过空格键在两张图片之间进行淡入淡出（crossfade）切换。

## Project Structure

```
akbox/
├── Cargo.toml          # workspace root, members = ["crates/*"]
├── crates/
│   └── image-viewer/
│       ├── Cargo.toml  # package, depends on eframe, image, etc.
│       └── src/
│           └── main.rs
```

## Tech Stack

- **eframe** (egui with wgpu backend) — native window + rendering
- **image** crate — decode PNG/JPEG/BMP/GIF/WebP
- **rfd** (optional) — file dialog fallback

## Core Logic

### State Machine

```
Idle(displaying img1) ──space pressed──▶ Transitioning(FadeTo2)
                                              │
Transitioning(FadeTo1) ◀──space released──┘
```

- **Idle**: displays current image (default img1). When img2 is loaded, listens for space key.
- **Transitioning**: computes alpha blend ratio based on elapsed time. Transition duration: **0.3s**. Easing: **ease-in-out** (smooth start and stop).

### Rendering

- During transition, draw both textures with alpha blending for crossfade effect.
- Drive animation frames via `ctx.request_repaint()`.
- Idle state: `request_repaint` is NOT called, so no unnecessary redraws.

### Drag-and-Drop

- Listen to `ctx.input().raw.dropped_files` for file drops.
- First drop → load as img1, display immediately.
- Second drop → load as img2, keep displaying current image.
- Subsequent drops → overwrite img2.
- Only accept image files (check by extension or load attempt).

### Image Display

- Aspect-ratio preserving fit within the window.
- Dark background behind the image (egui style or custom painting).

### UI Layout

- Window title: `Akbox Image Viewer`
- No image loaded: centered hint text "拖放图片到此处" (Drag images here)
- Images loaded: bottom status bar showing current image index and filename (if available)

## File Support

- PNG, JPEG, BMP, GIF (first frame), WebP, TIFF
- Large images handled efficiently via `image` crate's lazy loading

## Non-Goals

- Zoom / pan (can be added later)
- Fullscreen mode (can be added later)
- Keyboard shortcuts beyond space (can be added later)
- EXIF rotation (can be added later)

## Testing

- Manual testing on Windows: drag images from explorer, press/hold/release space.
- Verify crossfade smoothness and image aspect ratio correctness.
