use hound::{WavSpec, WavWriter};
use std::fs;
use std::path::{Path, PathBuf};

use crate::wav_reader::{PcmData, read_wav};

pub fn split_wav(path: &Path, gain_db: f64) -> Result<Vec<PathBuf>, String> {
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
            split_and_write_i16(&samples, channels, &out_dir, &stem, ch_width, mono, gain_db)
        }
        PcmData::Integer32 { samples, .. } => {
            split_and_write_i32(&samples, channels, &out_dir, &stem, ch_width, mono, gain_db)
        }
        PcmData::Float { samples, .. } => {
            split_and_write_f32(&samples, channels, &out_dir, &stem, ch_width, mono, gain_db)
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
    gain_db: f64,
) -> Result<Vec<PathBuf>, String> {
    let bufs = split_interleaved(samples, channels);
    let mut written = Vec::with_capacity(bufs.len());

    for (i, buf) in bufs.iter().enumerate() {
        let out_path = out_dir.join(format!("{stem}_ch{:0width$}.wav", i + 1, width = ch_width));
        write_channel_i16(&out_path, mono, buf, gain_db)?;
        written.push(out_path);
    }
    Ok(written)
}

fn split_and_write_i32(
    samples: &[i32],
    channels: usize,
    out_dir: &Path,
    stem: &str,
    ch_width: usize,
    mono: WavSpec,
    gain_db: f64,
) -> Result<Vec<PathBuf>, String> {
    let bufs = split_interleaved(samples, channels);
    let mut written = Vec::with_capacity(bufs.len());

    for (i, buf) in bufs.iter().enumerate() {
        let out_path = out_dir.join(format!("{stem}_ch{:0width$}.wav", i + 1, width = ch_width));
        write_channel_i32(&out_path, mono, buf, gain_db)?;
        written.push(out_path);
    }
    Ok(written)
}

fn split_and_write_f32(
    samples: &[f32],
    channels: usize,
    out_dir: &Path,
    stem: &str,
    ch_width: usize,
    mono: WavSpec,
    gain_db: f64,
) -> Result<Vec<PathBuf>, String> {
    let bufs = split_interleaved(samples, channels);
    let mut written = Vec::with_capacity(bufs.len());

    for (i, buf) in bufs.iter().enumerate() {
        let out_path = out_dir.join(format!("{stem}_ch{:0width$}.wav", i + 1, width = ch_width));
        write_channel_f32(&out_path, mono, buf, gain_db)?;
        written.push(out_path);
    }
    Ok(written)
}

fn write_channel_i16(
    path: &Path,
    spec: WavSpec,
    samples: &[i16],
    gain_db: f64,
) -> Result<(), String> {
    let linear = gain_linear(gain_db);
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        let sample = if gain_db == 0.0 {
            s
        } else {
            (f64::from(s) * linear)
                .round()
                .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
        };
        writer.write_sample(sample).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(())
}

fn write_channel_i32(
    path: &Path,
    spec: WavSpec,
    samples: &[i32],
    gain_db: f64,
) -> Result<(), String> {
    let linear = gain_linear(gain_db);
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        let sample = if gain_db == 0.0 {
            s
        } else {
            (f64::from(s) * linear)
                .round()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
        };
        writer.write_sample(sample).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(())
}

fn write_channel_f32(
    path: &Path,
    spec: WavSpec,
    samples: &[f32],
    gain_db: f64,
) -> Result<(), String> {
    let linear = gain_linear(gain_db) as f32;
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        let sample = if gain_db == 0.0 {
            s
        } else {
            (s * linear).clamp(-1.0, 1.0)
        };
        writer.write_sample(sample).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;
    Ok(())
}

fn gain_linear(gain_db: f64) -> f64 {
    10_f64.powf(gain_db / 20.0)
}
