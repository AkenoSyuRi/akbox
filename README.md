# Akbox

A collection of Rust tools in one workspace.

## Crates

| Crate | Description |
|---|---|
| `image-viewer` | Minimal image viewer with egui (eframe) |
| `multi2mono` | Split multi-channel WAV into individual mono files |
| `adb-wrapper` | Transparent adb proxy — auto-selects device on multi-device setups |

## Workspace Commands

```sh
cargo check --workspace
cargo test --workspace
cargo build --release --workspace
```

Use the per-crate commands below when working on one tool. More detailed usage notes live in each crate's README where available, such as `crates/adb-wrapper/README.md` and `crates/multi2mono/README.md`.

---

## image-viewer

A minimal image viewer built with [egui](https://github.com/emilk/egui) (eframe).

### Features

- **Drag & drop** images into the window to load (up to 9)
- **Thumbnail panel** on the right — current image highlighted with a red border and always centered
- **Click** a thumbnail to switch; click the **✕ icon** in its top-right corner to remove
- **Mouse wheel** anywhere in the window to switch images (1 tick = 1 image, no wrap-around)
- **Number keys** `1`–`9` to jump to a specific image
- **Space bar** — hold to crossfade between current and next image

### Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `eframe` | 0.31 | Native window + egui rendering |
| `image` | 0.25 | Image decoding (PNG, JPEG, etc.) |
| `ctrlc` | 3.4 | Ctrl-C signal handler |

### Build & Run

```sh
cargo run --manifest-path crates/image-viewer/Cargo.toml
cargo check --manifest-path crates/image-viewer/Cargo.toml
```

### Project Layout

```
crates/image-viewer/src/
├── main.rs     — Entry point, window setup
├── app.rs      — ImageViewer struct, image loading, UI rendering
└── layout.rs   — Aspect-ratio fitting and animation easing helpers
```

---

## multi2mono

将多通道 WAV 拆分为单声道 WAV 的命令行工具。

### Features

- 拖放 WAV 文件到可执行文件上即可处理
- 在源文件同目录下创建 `out` 文件夹
- 每个通道导出为独立单声道文件：`{文件名}_ch01.wav` …
- 支持 8/16/24/32 位整数 PCM 与 32 位浮点 WAV
- 自动修正错误的 `fmt` 头

### Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `hound` | 3.5 | WAV 写入 |

### Build & Run

```sh
cargo run --manifest-path crates/multi2mono/Cargo.toml -- <wav文件>
cargo check --manifest-path crates/multi2mono/Cargo.toml
```

### Project Layout

```
crates/multi2mono/src/
├── main.rs        — CLI 入口，文件拆分逻辑
└── wav_reader.rs  — 自定义 WAV 解析器（含 fmt 块修复）
```

---

## adb-wrapper

Rust 实现的 `adb` 透明代理，多设备时自动选择设备。

### Features

- 单设备或已指定 `-s`/`-d`/`-e`：直接透传给官方 adb
- 多设备 + 交互终端：弹出设备选择列表
- 多设备 + 非交互环境：报错并要求显式指定设备
- 三种选择器：`inquire`（方向键）、`skim`（模糊搜索）、`fzf`
- Shell 补全（bash / zsh / fish / PowerShell）
- `--wrapper-info` 诊断命令

### Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `anyhow` | 1 | 错误处理 |
| `ctrlc` | 3 | Ctrl-C 信号 |
| `dirs` | 6 | 配置文件路径 |
| `inquire` | 0.7 | 设备选择器 |
| `is-terminal` | 0.4 | TTY 检测 |
| `serde` | 1 | 配置序列化 |
| `skim` | 4.6 | 内置模糊选择器 |
| `toml` | 0.8 | 配置文件解析 |

### Build & Run

```sh
cargo run --manifest-path crates/adb-wrapper/Cargo.toml -- <adb参数>
cargo test --manifest-path crates/adb-wrapper/Cargo.toml
```

### Project Layout

```
crates/adb-wrapper/
├── src/
│   ├── main.rs         — 二进制入口
│   ├── lib.rs          — 核心逻辑
│   ├── adb.rs          — 执行官方 adb、设备列表
│   ├── args.rs         — 命令行参数预处理
│   ├── completions.rs  — Shell 补全脚本生成
│   ├── config.rs       — TOML 配置读写
│   ├── device.rs       — 设备数据结构
│   ├── diagnostics.rs  — --wrapper-info 诊断
│   ├── path.rs         — 真实 adb 路径解析
│   ├── selector.rs     — 设备选择器（inquire/skim/fzf）
│   └── tty.rs          — 终端交互检测
└── tests/
    ├── integration.rs  — 集成测试
    └── fixtures/       — mock adb 脚本
```
