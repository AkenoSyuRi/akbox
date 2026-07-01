use hound::{SampleFormat, WavSpec};
use std::path::Path;

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

/// KSDATAFORMAT_SUBTYPE_PCM
const SUBFORMAT_PCM: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38,
    0x9B, 0x71,
];
/// KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
const SUBFORMAT_IEEE_FLOAT: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38,
    0x9B, 0x71,
];

#[derive(Clone, Copy, Debug)]
struct FmtChunk {
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
}

/// 交错 PCM：`channels * bytes_per_sample` 字节为一帧，帧内依次为各声道同一时间点的样本。
pub enum PcmData {
    Integer16 {
        spec: WavSpec,
        samples: Vec<i16>,
    },
    Integer32 {
        spec: WavSpec,
        samples: Vec<i32>,
    },
    Float {
        spec: WavSpec,
        samples: Vec<f32>,
    },
}

impl PcmData {
    pub fn spec(&self) -> WavSpec {
        match self {
            PcmData::Integer16 { spec, .. }
            | PcmData::Integer32 { spec, .. }
            | PcmData::Float { spec, .. } => *spec,
        }
    }

    pub fn channels(&self) -> usize {
        self.spec().channels as usize
    }
}

pub fn read_wav(path: &Path) -> Result<PcmData, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    parse_wav(&bytes)
}

fn parse_wav(bytes: &[u8]) -> Result<PcmData, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("不是有效的 WAVE 文件".into());
    }

    let mut fmt: Option<FmtChunk> = None;
    let mut data: Option<&[u8]> = None;
    let mut offset = 12usize;

    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        let end = offset.saturating_add(size).min(bytes.len());
        let chunk = &bytes[offset..end];

        match id {
            b"fmt " => fmt = Some(parse_fmt(chunk)?),
            b"data" => data = Some(chunk),
            _ => {}
        }

        offset = end + (size % 2);
    }

    let fmt = fmt.ok_or("缺少 fmt 块")?;
    let data = data.ok_or("缺少 data 块")?;

    decode_pcm(fmt, data)
}

fn parse_fmt(chunk: &[u8]) -> Result<FmtChunk, String> {
    if chunk.len() < 16 {
        return Err("fmt 块过短".into());
    }
    let audio_format = u16::from_le_bytes([chunk[0], chunk[1]]);
    Ok(FmtChunk {
        audio_format: resolve_audio_format(audio_format, chunk)?,
        channels: u16::from_le_bytes([chunk[2], chunk[3]]),
        sample_rate: u32::from_le_bytes(chunk[4..8].try_into().unwrap()),
        bits_per_sample: u16::from_le_bytes([chunk[14], chunk[15]]),
    })
}

fn resolve_audio_format(audio_format: u16, chunk: &[u8]) -> Result<u16, String> {
    match audio_format {
        WAVE_FORMAT_PCM | WAVE_FORMAT_IEEE_FLOAT => Ok(audio_format),
        WAVE_FORMAT_EXTENSIBLE => {
            if chunk.len() < 40 {
                return Err("WAVE_FORMAT_EXTENSIBLE fmt 块过短".into());
            }
            let subformat = &chunk[24..40];
            if subformat == SUBFORMAT_PCM {
                Ok(WAVE_FORMAT_PCM)
            } else if subformat == SUBFORMAT_IEEE_FLOAT {
                Ok(WAVE_FORMAT_IEEE_FLOAT)
            } else {
                Err(format!(
                    "不支持的 extensible 子格式 GUID: {}",
                    subformat
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join("")
                ))
            }
        }
        other => Err(format!("不支持的 audio_format: {other}")),
    }
}

fn bytes_per_sample(bits: u16) -> u16 {
    bits.div_ceil(8)
}

fn decode_pcm(fmt: FmtChunk, data: &[u8]) -> Result<PcmData, String> {
    if fmt.channels == 0 {
        return Err("通道数为 0".into());
    }

    let sample_bytes = bytes_per_sample(fmt.bits_per_sample) as usize;
    let channels = fmt.channels as usize;
    let frame_bytes = channels.checked_mul(sample_bytes).ok_or("帧大小溢出")?;
    if frame_bytes == 0 || !data.len().is_multiple_of(frame_bytes) {
        return Err(format!(
            "data 长度 {} 不是交错帧大小 {}（{} 声道 × {} 字节/样本）的整数倍",
            data.len(),
            frame_bytes,
            channels,
            sample_bytes
        ));
    }

    let spec = wav_spec(&fmt)?;

    match (fmt.audio_format, fmt.bits_per_sample, spec.sample_format) {
        (WAVE_FORMAT_PCM, 16, SampleFormat::Int) => {
            let samples = decode_i16_le(data)?;
            Ok(PcmData::Integer16 { spec, samples })
        }
        (WAVE_FORMAT_PCM, 8, SampleFormat::Int) => {
            let samples = decode_i16_from_bytes(data, 1)?;
            Ok(PcmData::Integer16 { spec, samples })
        }
        (WAVE_FORMAT_PCM, 24, SampleFormat::Int) | (WAVE_FORMAT_PCM, 32, SampleFormat::Int) => {
            let bps = bytes_per_sample(fmt.bits_per_sample) as usize;
            let samples = decode_int32_le(data, bps, fmt.bits_per_sample)?;
            Ok(PcmData::Integer32 { spec, samples })
        }
        (WAVE_FORMAT_IEEE_FLOAT, 32, SampleFormat::Float) => {
            let samples = decode_f32_le(data)?;
            Ok(PcmData::Float { spec, samples })
        }
        (WAVE_FORMAT_PCM, bits, SampleFormat::Int) if bits % 8 == 0 => {
            let bps = bytes_per_sample(bits) as usize;
            let samples = decode_int32_le(data, bps, bits)?;
            Ok(PcmData::Integer32 { spec, samples })
        }
        (WAVE_FORMAT_IEEE_FLOAT, 64, SampleFormat::Float) => Err("暂不支持 64 位浮点 WAV".into()),
        (af, bits, _) => Err(format!(
            "不支持的 WAV 格式: audio_format={af}, bits_per_sample={bits}"
        )),
    }
}

fn wav_spec(fmt: &FmtChunk) -> Result<WavSpec, String> {
    let sample_format = match fmt.audio_format {
        WAVE_FORMAT_PCM => SampleFormat::Int,
        WAVE_FORMAT_IEEE_FLOAT => SampleFormat::Float,
        other => return Err(format!("不支持的 audio_format: {other}")),
    };
    Ok(WavSpec {
        channels: fmt.channels,
        sample_rate: fmt.sample_rate,
        bits_per_sample: fmt.bits_per_sample,
        sample_format,
    })
}

fn decode_i16_le(data: &[u8]) -> Result<Vec<i16>, String> {
    if data.len() % 2 != 0 {
        return Err("16-bit 样本数据长度不是偶数".into());
    }
    Ok(data
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect())
}

fn decode_i16_from_bytes(data: &[u8], bytes_per_sample: usize) -> Result<Vec<i16>, String> {
    if data.len() % bytes_per_sample != 0 {
        return Err("样本数据长度无效".into());
    }
    Ok(data
        .chunks_exact(bytes_per_sample)
        .map(|c| match bytes_per_sample {
            1 => (c[0] as i8 as i16) * 256,
            2 => i16::from_le_bytes([c[0], c[1]]),
            _ => unreachable!(),
        })
        .collect())
}

fn decode_f32_le(data: &[u8]) -> Result<Vec<f32>, String> {
    if data.len() % 4 != 0 {
        return Err("32-bit 浮点样本数据长度无效".into());
    }
    Ok(data
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

fn decode_int32_le(data: &[u8], bytes_per_sample: usize, bits: u16) -> Result<Vec<i32>, String> {
    if data.len() % bytes_per_sample != 0 {
        return Err("样本数据长度无效".into());
    }
    Ok(data
        .chunks_exact(bytes_per_sample)
        .map(|c| sample_to_i32(c, bits))
        .collect())
}

fn sample_to_i32(bytes: &[u8], bits: u16) -> i32 {
    match bytes.len() {
        3 => {
            let v = i32::from_le_bytes([
                bytes[0],
                bytes[1],
                bytes[2],
                if bytes[2] & 0x80 != 0 { 0xFF } else { 0 },
            ]);
            v >> (32 - bits)
        }
        4 => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        2 => i32::from(i16::from_le_bytes([bytes[0], bytes[1]])),
        1 => i32::from(bytes[0] as i8),
        _ => 0,
    }
}
