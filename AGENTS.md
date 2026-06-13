# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace defined by `Cargo.toml` with members under `crates/*`.

- `crates/image-viewer`: native egui/eframe image viewer. UI state lives in `src/app.rs`, layout helpers in `src/layout.rs`, and window setup in `src/main.rs`.
- `crates/multi2mono`: CLI tool for splitting multi-channel WAV files. WAV parsing is in `src/wav_reader.rs`; command flow is in `src/main.rs`.
- `crates/adb-wrapper`: transparent `adb` proxy. Core logic is split across `src/lib.rs`, `src/adb.rs`, `src/selector.rs`, `src/config.rs`, and related focused modules.
- `crates/adb-wrapper/tests`: integration tests and mock adb fixtures.
- `docs/superpowers`: design specs and implementation plans. Treat these as historical/project context, not runtime assets.

## Build, Test, and Development Commands

Prefix shell commands with `rtk` when working in this repository.

- `rtk cargo check --workspace`: type-check all crates.
- `rtk cargo test --workspace`: run the workspace test suite.
- `rtk cargo test --manifest-path crates/adb-wrapper/Cargo.toml`: run adb-wrapper integration tests.
- `rtk cargo run --manifest-path crates/image-viewer/Cargo.toml`: launch the image viewer.
- `rtk cargo run --manifest-path crates/multi2mono/Cargo.toml -- <file.wav>`: split a WAV file.
- `rtk cargo run --manifest-path crates/adb-wrapper/Cargo.toml -- <adb args>`: run the adb proxy locally.

## Coding Style & Naming Conventions

Use Rust 2024 edition conventions. Format with `rtk cargo fmt --all` before submitting broad changes. Prefer small, focused modules like the existing `adb-wrapper/src/*.rs` layout. Use `snake_case` for functions, modules, and variables; `PascalCase` for types; and `SCREAMING_SNAKE_CASE` for constants.

Keep changes surgical. Do not refactor adjacent code unless it directly supports the requested change.

## Testing Guidelines

Use Rust’s built-in test framework. Place integration tests under each crate’s `tests/` directory and keep fixtures near the tests that consume them, as in `crates/adb-wrapper/tests/fixtures`. Name tests after the behavior being verified, not the implementation detail.

Run the narrowest relevant test first, then `rtk cargo test --workspace` when the change can affect shared workspace behavior.

## Commit & Pull Request Guidelines

Recent history uses Conventional Commit-style subjects such as `feat: ...`, `fix: ...`, `docs: ...`, and `refactor: ...`. Keep the subject imperative and scoped to the changed behavior.

Pull requests should include a short description, the affected crate(s), verification commands run, and screenshots or recordings for visible `image-viewer` UI changes.

## Agent-Specific Instructions

Reply in Simplified Chinese during collaboration. Use `uv run` for any Python command. Surface assumptions before implementation, prefer the simplest working change, and verify results explicitly before reporting completion.
