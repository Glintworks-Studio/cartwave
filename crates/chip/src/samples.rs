//! Synthesised DPCM samples (our own; nothing ripped), delta-encoded for the
//! DMC and laid out in its 16 KiB ROM at $C000.
//!
//! Each waveform is generated at the DMC output rate it will play at, then
//! encoded greedily against the 7-bit counter: a 1 bit if the target is above
//! the current level (+2), else a 0 bit (-2), starting from `START_LEVEL`.

use crate::dmc::ROM_SIZE;
use crate::math;
use crate::tables::{CPU_HZ_NTSC, DMC_RATE_NTSC};

/// Built-in samples plus user slots.
pub const SAMPLE_COUNT: usize = BUILTIN_COUNT + USER_SLOTS;
pub const BUILTIN_COUNT: usize = 4;
/// Imported (user) samples, after the built-ins.
pub const USER_SLOTS: usize = 4;
pub const START_LEVEL: u8 = 64;
pub const NAMES: [&str; SAMPLE_COUNT] = ["Kick", "Snare", "Tom", "Crash", "User 1", "User 2", "User 3", "User 4"];
/// ROM offset where user samples start (the built-ins use ~5 KiB below it).
pub const USER_BASE: usize = 0x1800;
/// Bytes available for user samples.
pub const USER_ROM: usize = crate::dmc::ROM_SIZE - USER_BASE;
/// The longest sample the DMC can play: $4013 = 255 -> 16 * 255 + 1 bytes.
pub const MAX_SAMPLE_BYTES: usize = 16 * 255 + 1;

/// Which sample a note plays: pitches 36..44 pick one directly (the piano
/// roll's DPCM rows), others cycle through the built-ins (pitch % 4).
pub fn sample_for_pitch(pitch: u8) -> usize {
    if (36..36 + SAMPLE_COUNT as u8).contains(&pitch) {
        (pitch - 36) as usize
    } else {
        pitch as usize % BUILTIN_COUNT
    }
}

/// Delta-encodes `pcm` (-1..1, already at the DMC rate) the way the built-ins
/// are: a 1 bit if the target is above the counter, else 0, from
/// `START_LEVEL`. Pads to a playable length (16 l + 1 bytes) with a
/// return-to-centre tail. Returns the bytes; at most `MAX_SAMPLE_BYTES`.
pub fn encode_pcm(pcm: &[f32], out: &mut [u8]) -> usize {
    let amp = 60.0f32;
    let mut level = START_LEVEL as i32;
    let max = out.len().min(MAX_SAMPLE_BYTES);
    let want = pcm.len().div_ceil(8);
    let l = (want.saturating_sub(1).div_ceil(16)).clamp(0, 255);
    let len = (16 * l + 1).min(max);
    for (byte_i, byte) in out[..len].iter_mut().enumerate() {
        let mut b = 0u8;
        for bit in 0..8 {
            let n = byte_i * 8 + bit;
            let target = START_LEVEL as f32 + amp * pcm.get(n).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
            if (level as f32) < target {
                b |= 1 << bit;
                if level <= 125 {
                    level += 2;
                }
            } else if level >= 2 {
                level -= 2;
            }
        }
        *byte = b;
    }
    len
}
/// The rate index each sample was synthesised for.
pub const NATIVE_RATE: u8 = 15;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SampleRef {
    /// $4012 value: address = $C000 + 64 * a.
    pub addr_reg: u8,
    /// $4013 value: length = 16 * l + 1 bytes.
    pub len_reg: u8,
    /// The rate index the sample was encoded for (user samples); `None`:
    /// the lane's Rate control.
    pub rate: Option<u8>,
    /// Slot holds a sample.
    pub present: bool,
}

/// Fills `rom` with the sample set and returns where each one is.
pub fn build(rom: &mut [u8; ROM_SIZE]) -> [SampleRef; SAMPLE_COUNT] {
    let fs = CPU_HZ_NTSC / DMC_RATE_NTSC[NATIVE_RATE as usize] as f64;
    let mut refs = [SampleRef::default(); SAMPLE_COUNT];
    let mut offset = 0usize;
    for (i, r) in refs.iter_mut().take(BUILTIN_COUNT).enumerate() {
        let secs = [0.22, 0.16, 0.30, 0.55][i];
        // Bytes: 16 l + 1, enough for `secs` at 8 bits per byte.
        let bits = (secs * fs) as usize;
        let l = ((bits / 8).saturating_sub(1) / 16 + 1).min(255);
        let len = 16 * l + 1;
        assert!(offset + len <= ROM_SIZE);
        encode(i, fs, &mut rom[offset..offset + len]);
        *r = SampleRef {
            addr_reg: (offset / 64) as u8,
            len_reg: l as u8,
            rate: None,
            present: true,
        };
        offset = (offset + len).div_ceil(64) * 64;
    }
    assert!(offset <= USER_BASE, "built-in samples overflow into the user area");
    refs
}

/// Target level (around 0, +/-1 = full scale) of sample `which` at time `t` s.
fn waveform(which: usize, t: f64, noise: &mut u32) -> f64 {
    let mut white = || {
        *noise ^= *noise << 13;
        *noise ^= *noise >> 17;
        *noise ^= *noise << 5;
        (*noise as f64 / u32::MAX as f64) * 2.0 - 1.0
    };
    match which {
        // Kick: sine with an exponential pitch drop 160 -> 45 Hz.
        0 => {
            let f0 = 160.0;
            let f1 = 45.0;
            let k = 25.0;
            // Phase = integral of f1 + (f0 - f1) e^{-k t}.
            let phase = f1 * t + (f0 - f1) * (1.0 - math::exp(-k * t)) / k;
            math::sin(2.0 * math::PI * phase) * math::exp(-t * 14.0)
        }
        // Snare: noise burst plus a 190 Hz body.
        1 => {
            let body = math::sin(2.0 * math::PI * 190.0 * t) * math::exp(-t * 30.0);
            (0.75 * white() * math::exp(-t * 18.0) + 0.5 * body).clamp(-1.0, 1.0)
        }
        // Tom: sine 130 -> 85 Hz.
        2 => {
            let phase = 85.0 * t + 45.0 * (1.0 - math::exp(-8.0 * t)) / 8.0;
            math::sin(2.0 * math::PI * phase) * math::exp(-t * 9.0)
        }
        // Crash: long noise decay.
        _ => white() * math::exp(-t * 6.0) * 0.8,
    }
}

fn encode(which: usize, fs: f64, out: &mut [u8]) {
    let mut level = START_LEVEL as i32;
    let mut noise = 0x1234_5678u32 + which as u32;
    let amp = 60.0; // levels: 64 +/- 60
    for (byte_i, byte) in out.iter_mut().enumerate() {
        let mut b = 0u8;
        for bit in 0..8 {
            let n = byte_i * 8 + bit;
            let t = n as f64 / fs;
            let target = START_LEVEL as f64 + amp * waveform(which, t, &mut noise);
            // The hardware clamps; mirror it so the encoder tracks reality.
            if (level as f64) < target {
                b |= 1 << bit;
                if level <= 125 {
                    level += 2;
                }
            } else if level >= 2 {
                level -= 2;
            }
        }
        *byte = b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dmc::Dmc;
    use std::boxed::Box;

    #[test]
    fn layout_fits_and_is_aligned() {
        let mut rom = Box::new([0u8; ROM_SIZE]);
        let refs = build(&mut rom);
        let mut end = 0;
        for r in refs.into_iter().filter(|r| r.present) {
            let addr = r.addr_reg as usize * 64;
            assert!(addr >= end, "no overlap");
            end = addr + 16 * r.len_reg as usize + 1;
        }
        assert!(end <= ROM_SIZE);
    }

    #[test]
    fn playback_ends_near_centre() {
        // Decoded through the real DMC, each sample decays back to ~64.
        let mut rom = Box::new([0u8; ROM_SIZE]);
        let refs = build(&mut rom);
        for r in refs.into_iter().filter(|r| r.present) {
            let mut d = Dmc::default();
            d.write(0, NATIVE_RATE);
            d.write(1, START_LEVEL);
            d.write(2, r.addr_reg);
            d.write(3, r.len_reg);
            d.set_enabled(true, &rom);
            let mut lo = 127;
            let mut hi = 0;
            let bits = (16 * r.len_reg as usize + 1) * 8 + 16;
            for _ in 0..bits {
                d.clock_timer(&rom);
                lo = lo.min(d.level);
                hi = hi.max(d.level);
            }
            assert!(!d.active());
            assert!((d.level as i32 - 64).abs() <= 8, "ends at {}", d.level);
            // Noise content is slew-limited (+/-2 per bit), so swings differ.
            assert!(hi - lo > 30, "has real swing: {lo}..{hi}");
        }
    }
}
