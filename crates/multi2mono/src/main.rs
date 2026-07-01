mod wav_reader;

use hound::{WavSpec, WavWriter};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use wav_reader::{PcmData, read_wav};

const HELP: &str = "\
multi2mono - 将多声道 WAV 拆分为单声道文件

用法:
    multi2mono <wav文件> [更多文件...]

    也可将 wav 文件拖放到本程序上。

输出:
    在与源文件同级的 out/ 目录下生成单声道文件。

选项:
    -h, --help    显示此帮助信息
";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{HELP}");
        return;
    }
    if args.is_empty() {
        eprint!("{HELP}");
        pause_on_windows();
        process::exit(1);
    }

    let mut had_error = false;
    for arg in &args {
        let path = PathBuf::from(trim_quotes(arg));
        if let Err(e) = split_wav(&path) {
            eprintln!("处理 {} 失败: {e}", path.display());
            had_error = true;
        }
    }

    if had_error {
        pause_on_windows();
        process::exit(1);
    }
}

fn trim_quotes(s: &str) -> &str {
    s.trim().trim_matches('"')
}

fn split_wav(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("不是有效文件: {}", path.display()));
    }

    let pcm = read_wav(path)?;
    let spec = pcm.spec();
    let channels = pcm.channels();

    let parent = path
        .parent()
        .ok_or_else(|| "无法确定文件所在目录".to_string())?;
    let out_dir = parent.join("out");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());

    let ch_width = channel_index_width(channels);
    let mono = mono_spec(spec);

    match pcm {
        PcmData::Integer16 { samples, .. } => {
            split_and_write_i16(&samples, channels, &out_dir, &stem, ch_width, mono)
        }
        PcmData::Integer32 { samples, .. } => {
            split_and_write_i32(&samples, channels, &out_dir, &stem, ch_width, mono)
        }
        PcmData::Float { samples, .. } => {
            split_and_write_f32(&samples, channels, &out_dir, &stem, ch_width, mono)
        }
    }
}

fn channel_index_width(channels: usize) -> usize {
    let digits = (channels as f64).log10().floor() as usize + 1;
    digits.max(2)
}

fn mono_spec(spec: WavSpec) -> WavSpec {
    WavSpec {
        channels: 1,
        ..spec
    }
}

/// 标准交错：每帧 `channels` 个样本，依次为各声道同一时间戳。
fn split_interleaved<T: Copy>(samples: &[T], channels: usize) -> Vec<Vec<T>> {
    let frames = samples.len() / channels;
    let mut bufs: Vec<Vec<T>> = vec![Vec::with_capacity(frames); channels];
    for frame in samples.chunks(channels) {
        for (ch, &sample) in frame.iter().enumerate() {
            bufs[ch].push(sample);
        }
    }
    bufs
}

fn split_and_write_i16(
    samples: &[i16],
    channels: usize,
    out_dir: &Path,
    stem: &str,
    ch_width: usize,
    mono: WavSpec,
) -> Result<(), String> {
    let bufs = split_interleaved(samples, channels);

    for (i, buf) in bufs.iter().enumerate() {
        let out_path = out_dir.join(format!("{stem}_ch{:0width$}.wav", i + 1, width = ch_width));
        write_channel_i16(&out_path, mono, buf)?;
        println!("已写入: {}", out_path.display());
    }
    Ok(())
}

fn split_and_write_i32(
    samples: &[i32],
    channels: usize,
    out_dir: &Path,
    stem: &str,
    ch_width: usize,
    mono: WavSpec,
) -> Result<(), String> {
    let bufs = split_interleaved(samples, channels);

    for (i, buf) in bufs.iter().enumerate() {
        let out_path = out_dir.join(format!("{stem}_ch{:0width$}.wav", i + 1, width = ch_width));
        write_channel_i32(&out_path, mono, buf)?;
        println!("已写入: {}", out_path.display());
    }
    Ok(())
}

fn split_and_write_f32(
    samples: &[f32],
    channels: usize,
    out_dir: &Path,
    stem: &str,
    ch_width: usize,
    mono: WavSpec,
) -> Result<(), String> {
    let bufs = split_interleaved(samples, channels);

    for (i, buf) in bufs.iter().enumerate() {
        let out_path = out_dir.join(format!("{stem}_ch{:0width$}.wav", i + 1, width = ch_width));
        write_channel_f32(&out_path, mono, buf)?;
        println!("已写入: {}", out_path.display());
    }
    Ok(())
}

fn write_channel_i16(path: &Path, spec: WavSpec, samples: &[i16]) -> Result<(), String> {
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        writer.write_sample(s).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(())
}

fn write_channel_i32(path: &Path, spec: WavSpec, samples: &[i32]) -> Result<(), String> {
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        writer.write_sample(s).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(())
}

fn write_channel_f32(path: &Path, spec: WavSpec, samples: &[f32]) -> Result<(), String> {
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        writer.write_sample(s).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn pause_on_windows() {
    use std::io::{self, Write};
    eprint!("按 Enter 键退出...");
    let _ = io::stderr().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

#[cfg(not(windows))]
fn pause_on_windows() {}
