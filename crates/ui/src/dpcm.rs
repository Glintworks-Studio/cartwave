//! Audio import for the DPCM channel: a WAV file becomes a 1-bit delta
//! sample, exactly what the 2A03's DMC plays. This is the only authentic way
//! to get recorded sound out of the chip, so it's the only audio import.
//!
//! The conversion picks the highest DMC rate at which the sound fits the
//! hardware's sample length limit (4081 bytes) and the free sample ROM:
//! short hits get 33 kHz, longer sounds drop to lower rates (and get the
//! lo-fi grit that implies). Mixed to mono, normalised, linearly resampled.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use chip::samples::{encode_pcm, SampleRef, MAX_SAMPLE_BYTES, USER_BASE, USER_ROM, USER_SLOTS};
use chip::tables::{CPU_HZ_NTSC, DMC_RATE_NTSC};
use engine::DpcmBank;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserSample {
    pub name: String,
    /// DMC rate index 0..=15 it was encoded for.
    pub rate: u8,
    /// Delta-encoded bytes (16 l + 1 long).
    pub bytes: Vec<u8>,
}

impl UserSample {
    pub fn hz(&self) -> f64 {
        rate_hz(self.rate)
    }

    pub fn seconds(&self) -> f64 {
        self.bytes.len() as f64 * 8.0 / self.hz()
    }
}

pub fn rate_hz(rate: u8) -> f64 {
    CPU_HZ_NTSC / DMC_RATE_NTSC[rate.min(15) as usize] as f64
}

/// Bytes a sample takes in the ROM (64-byte aligned).
fn footprint(len: usize) -> usize {
    len.div_ceil(64) * 64
}

/// ROM bytes still free for another sample.
pub fn free_bytes(samples: &[Option<UserSample>]) -> usize {
    USER_ROM - samples.iter().flatten().map(|s| footprint(s.bytes.len())).sum::<usize>()
}

/// The first empty user slot.
pub fn free_slot(samples: &[Option<UserSample>]) -> Option<usize> {
    (0..USER_SLOTS).find(|&i| samples.get(i).is_none_or(|s| s.is_none()))
}

/// Lays the samples out in the user ROM area for the engine.
pub fn bank(samples: &[Option<UserSample>]) -> Arc<DpcmBank> {
    let mut b = DpcmBank::default();
    let mut off = 0usize;
    for (i, s) in samples.iter().take(USER_SLOTS).enumerate() {
        let Some(s) = s else { continue };
        let len = s.bytes.len().min(MAX_SAMPLE_BYTES);
        if len == 0 || off + len > USER_ROM {
            continue;
        }
        b.data[off..off + len].copy_from_slice(&s.bytes[..len]);
        b.refs[i] = SampleRef {
            addr_reg: ((USER_BASE + off) / 64) as u8,
            len_reg: ((len - 1) / 16) as u8,
            rate: Some(s.rate),
            present: true,
        };
        off += footprint(len);
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    samples.hash(&mut h);
    // 0 is "nothing installed yet"; an empty bank must still clear old samples.
    b.version = h.finish() | 1;
    Arc::new(b)
}

/// Reads a WAV file: PCM 8/16/24/32-bit or float 32, any rate and channel
/// count. Returns mono samples (-1..1) and the sample rate.
pub fn read_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    let bad = || "Not a WAV file this can read (PCM or float).".to_string();
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(bad());
    }
    let (mut fmt, mut data) = (None, None);
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
        let body = bytes.get(i + 8..(i + 8 + len).min(bytes.len())).ok_or_else(bad)?;
        match id {
            b"fmt " => fmt = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        i += 8 + len + (len & 1);
    }
    let (fmt, data) = (fmt.ok_or_else(bad)?, data.ok_or_else(bad)?);
    if fmt.len() < 16 {
        return Err(bad());
    }
    let u16_at = |o: usize| u16::from_le_bytes([fmt[o], fmt[o + 1]]);
    let mut format = u16_at(0);
    let channels = u16_at(2).max(1) as usize;
    let rate = u32::from_le_bytes(fmt[4..8].try_into().unwrap());
    let bits = u16_at(14);
    if format == 0xFFFE && fmt.len() >= 26 {
        format = u16_at(24); // WAVE_FORMAT_EXTENSIBLE: the sub-format's first field
    }
    let width = (bits as usize).div_ceil(8);
    if width == 0 || rate == 0 {
        return Err(bad());
    }
    let frame = width * channels;
    let sample = |b: &[u8]| -> Option<f32> {
        Some(match (format, bits) {
            (1, 8) => (b[0] as f32 - 128.0) / 128.0,
            (1, 16) => i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0,
            (1, 24) => (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0,
            (1, 32) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2_147_483_648.0,
            (3, 32) => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            _ => return None,
        })
    };
    let mut out = Vec::with_capacity(data.len() / frame);
    for f in data.chunks_exact(frame) {
        let mut sum = 0.0;
        for c in 0..channels {
            sum += sample(&f[c * width..]).ok_or_else(|| format!("WAV format {format} at {bits} bits isn't supported."))?;
        }
        out.push(sum / channels as f32);
    }
    Ok((out, rate))
}

/// Converts audio into a DPCM sample that fits in `free` ROM bytes.
pub fn convert(name: &str, pcm: &[f32], sr: u32, free: usize) -> Result<(UserSample, bool), String> {
    if pcm.is_empty() {
        return Err("The audio file is empty.".into());
    }
    let limit = free.min(MAX_SAMPLE_BYTES);
    if limit < 17 {
        return Err("The DPCM sample memory is full: remove a sample first.".into());
    }
    let secs = pcm.len() as f64 / sr as f64;
    // Highest rate whose encoding fits; else the lowest rate, truncated.
    let rate = (0..16u8)
        .rev()
        .find(|&r| ((secs * rate_hz(r)) / 8.0).ceil() as usize + 16 <= limit)
        .unwrap_or(0);
    let truncated = ((secs * rate_hz(rate)) / 8.0).ceil() as usize + 16 > limit;
    // Remove DC, normalise to the encoder's range.
    let mean = pcm.iter().sum::<f32>() / pcm.len() as f32;
    let peak = pcm.iter().fold(0f32, |m, x| m.max((x - mean).abs())).max(1e-6);
    let gain = 0.95 / peak;
    let hz = rate_hz(rate);
    let n = ((secs * hz) as usize).min(limit * 8);
    let step = sr as f64 / hz;
    let resampled: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f64 * step;
            let j = t as usize;
            let fr = (t - j as f64) as f32;
            let a = pcm.get(j).copied().unwrap_or(0.0);
            let b = pcm.get(j + 1).copied().unwrap_or(a);
            ((a + (b - a) * fr) - mean) * gain
        })
        .collect();
    let mut bytes = vec![0u8; limit];
    let len = encode_pcm(&resampled, &mut bytes);
    bytes.truncate(len);
    Ok((UserSample { name: name.into(), rate, bytes }, truncated))
}

pub fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn from_hex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("Bad sample data in the project file.".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "Bad sample data in the project file.".to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav16(pcm: &[i16], rate: u32) -> Vec<u8> {
        let mut b = b"RIFF".to_vec();
        b.extend((36 + pcm.len() as u32 * 2).to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(rate.to_le_bytes());
        b.extend((rate * 2).to_le_bytes());
        b.extend(2u16.to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend((pcm.len() as u32 * 2).to_le_bytes());
        for s in pcm {
            b.extend(s.to_le_bytes());
        }
        b
    }

    #[test]
    fn short_hits_get_the_top_rate_long_sounds_a_lower_one() {
        let tone = |secs: f64| -> Vec<i16> {
            (0..(secs * 44100.0) as usize).map(|i| ((i as f64 * 0.05).sin() * 20000.0) as i16).collect()
        };
        let (pcm, sr) = read_wav(&wav16(&tone(0.3), 44100)).unwrap();
        assert_eq!(sr, 44100);
        let (s, cut) = convert("hit", &pcm, sr, USER_ROM).unwrap();
        assert_eq!((s.rate, cut), (15, false));
        assert_eq!((s.bytes.len() - 1) % 16, 0);
        let (pcm, sr) = read_wav(&wav16(&tone(3.0), 44100)).unwrap();
        let (l, cut) = convert("long", &pcm, sr, USER_ROM).unwrap();
        assert!(l.rate < 15 && !cut, "rate {}", l.rate);
        assert!(l.bytes.len() <= MAX_SAMPLE_BYTES);
        let b = bank(&[Some(s.clone()), None, Some(l)]);
        assert!(b.refs[0].present && !b.refs[1].present && b.refs[2].present);
        assert_eq!(from_hex(&to_hex(&s.bytes)).unwrap(), s.bytes);
    }
}
