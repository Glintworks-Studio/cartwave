//! File builders shared by the integration tests.
#![allow(dead_code)]

pub fn var(mut n: u32) -> Vec<u8> {
    let mut b = vec![(n & 0x7F) as u8];
    n >>= 7;
    while n > 0 {
        b.insert(0, (n & 0x7F) as u8 | 0x80);
        n >>= 7;
    }
    b
}

/// A track from (delta, event bytes); adds end-of-track.
pub fn trk(events: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut d = Vec::new();
    for (dt, e) in events {
        d.extend(var(*dt));
        d.extend(e);
    }
    d.extend([0, 0xFF, 0x2F, 0]);
    d
}

pub fn smf(format: u16, ppq: u16, tracks: &[Vec<u8>]) -> Vec<u8> {
    let mut f = b"MThd".to_vec();
    f.extend(6u32.to_be_bytes());
    f.extend(format.to_be_bytes());
    f.extend((tracks.len() as u16).to_be_bytes());
    f.extend(ppq.to_be_bytes());
    for t in tracks {
        f.extend(b"MTrk");
        f.extend((t.len() as u32).to_be_bytes());
        f.extend(t);
    }
    f
}

pub fn name(n: &str) -> Vec<u8> {
    let mut e = vec![0xFF, 0x03];
    e.extend(var(n.len() as u32));
    e.extend(n.as_bytes());
    e
}

pub fn tempo(bpm: f64) -> Vec<u8> {
    let us = (60_000_000.0 / bpm) as u32;
    vec![0xFF, 0x51, 3, (us >> 16) as u8, (us >> 8) as u8, us as u8]
}

/// A melody on `ch`: (delta, len, pitch) steps.
pub fn melody(ch: u8, notes: &[(u32, u32, u8)]) -> Vec<(u32, Vec<u8>)> {
    let mut ev = Vec::new();
    for &(dt, len, p) in notes {
        ev.push((dt, vec![0x90 | ch, p, 100]));
        ev.push((len, vec![0x80 | ch, p, 0]));
    }
    ev
}

/// A WAV file: `format` 1 (PCM) or 3 (float), or 0xFFFE (extensible, PCM).
pub fn wav(format: u16, channels: u16, rate: u32, bits: u16, frames: &[f32]) -> Vec<u8> {
    let width = bits as usize / 8;
    let mut data = Vec::new();
    for &x in frames {
        for c in 0..channels {
            let x = if c % 2 == 1 { -x } else { x };
            match (format, bits) {
                (3, 32) => data.extend(x.to_le_bytes()),
                (_, 8) => data.push(((x.clamp(-1.0, 1.0) * 127.0) + 128.0) as u8),
                (_, 16) => data.extend(((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()),
                (_, 24) => data.extend(&((x.clamp(-1.0, 1.0) * 8_388_607.0) as i32).to_le_bytes()[..3]),
                _ => data.extend(((x.clamp(-1.0, 1.0) as f64 * 2_147_483_647.0) as i32).to_le_bytes()),
            }
        }
    }
    let mut fmt = Vec::new();
    fmt.extend(format.to_le_bytes());
    fmt.extend(channels.to_le_bytes());
    fmt.extend(rate.to_le_bytes());
    fmt.extend((rate * (channels as u32) * width as u32).to_le_bytes());
    fmt.extend((channels * width as u16).to_le_bytes());
    fmt.extend(bits.to_le_bytes());
    if format == 0xFFFE {
        fmt.extend(22u16.to_le_bytes());
        fmt.extend(bits.to_le_bytes());
        fmt.extend(0u32.to_le_bytes());
        fmt.extend(1u16.to_le_bytes()); // KSDATAFORMAT_SUBTYPE_PCM's first field
        fmt.extend([0u8; 14]);
    }
    let mut b = b"RIFF".to_vec();
    b.extend(((4 + 8 + fmt.len() + 8 + data.len() + (data.len() & 1)) as u32).to_le_bytes());
    b.extend(b"WAVE");
    // An unrelated chunk first, with an odd length (pad byte).
    b.extend(b"LIST");
    b.extend(3u32.to_le_bytes());
    b.extend([b'a', b'b', b'c', 0]);
    b.extend(b"fmt ");
    b.extend((fmt.len() as u32).to_le_bytes());
    b.extend(&fmt);
    b.extend(b"data");
    b.extend((data.len() as u32).to_le_bytes());
    b.extend(&data);
    if data.len() & 1 == 1 {
        b.push(0);
    }
    b
}

pub fn tone(secs: f64, rate: u32) -> Vec<f32> {
    (0..(secs * rate as f64) as usize).map(|i| (i as f32 * 0.03).sin() * 0.8).collect()
}
