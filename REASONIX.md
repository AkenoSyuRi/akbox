# Akbox — Image Viewer

## Stack
- **Language:** Rust (edition 2024)
- **Framework:** `eframe 0.31` (egui-based native window)
- **Image loading:** `image 0.25`
- **Signal handling:** `ctrlc 3.4` (Ctrl-C to exit)

## Layout
- `Cargo.toml` — workspace root, members = `["crates/*"]`
- `crates/image-viewer/` — single crate (binary)
- `crates/image-viewer/src/main.rs` — entire app (~400 lines)
- `crates/image-viewer/Cargo.toml` — crate manifest
- `docs/superpowers/` — historical design docs (not code)
- `src/` — empty (workspace doesn't use a root crate)

## Commands
- **Build/Check:** `cargo check --manifest-path crates/image-viewer/Cargo.toml`
- **Run:** `cargo run --manifest-path crates/image-viewer/Cargo.toml`
- **No tests, no lint/format config, no scripts block**

## Conventions
- `fit_rect()` and `ease_in_out()` are free functions in `main.rs`
- Thumbnail interactions are deferred via `ThumbnailAction` enum (collected during render, applied after panel closes)
- Scroll wheel uses a 150ms cooldown timer (`scroll_cooldown: f32`) for 1-image-per-tick behavior

## Watch out for
- `multi_edit` / `edit_file` SEARCH must match byte-for-byte — always `read_file` first
- `run_command` rejects `cd` — use `cargo --manifest-path <path>` instead
- No README — start here
