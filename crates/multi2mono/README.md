# multi2mono

将多通道 WAV 拆分为单声道 WAV 的命令行工具。

> 本 crate 是 [akbox](https://github.com/your/akbox) workspace 成员，位于 `crates/multi2mono/`。

## 功能

- 支持拖放：把 WAV 文件拖到 `multi2mono.exe` 上即可处理
- 在源文件同目录下创建 `out` 文件夹
- 每个通道导出为独立单声道文件：`{文件名}_ch01.wav`、`{文件名}_ch02.wav` …
- 支持 8/16/24/32 位整数 PCM 与 32 位浮点 WAV
- 自动修正错误的 `fmt` 头（例如通道数与 `block_align` 不一致）；PCM 仍按 **标准 WAV 交错帧**解析
- **交错帧**：每帧为连续的 `声道数 × 每样本字节数`（例如 43 声道 × int16 ⇒ 每帧 43 个 short），帧内依次为 channel0…channelN−1 在同一时刻的样本

## 使用

```powershell
# 拖放 wav 到 multi2mono.exe

# 命令行
cargo run --manifest-path crates/multi2mono/Cargo.toml -- "D:\audio\recording.wav"
cargo run --manifest-path crates/multi2mono/Cargo.toml -- file1.wav file2.wav
```

示例：`D:\audio\stereo.wav` → `D:\audio\out\stereo_ch01.wav`、`stereo_ch02.wav`

## 构建

需要 [Rust](https://www.rust-lang.org/) 工具链。

```powershell
# 单独编译本 crate
cargo build --release --manifest-path crates/multi2mono/Cargo.toml

# 或全 workspace 编译
cargo build --release
```

可执行文件：`target\release\multi2mono.exe`

## 依赖

- [hound](https://crates.io/crates/hound) — WAV 写入

读取由项目内 `wav_reader` 完成：修补常见错误 `fmt` 后按交错 PCM 解码（与 `wave`/DAW 的多声道 WAV 约定一致）。

## 许可

见仓库许可文件（如有）。
