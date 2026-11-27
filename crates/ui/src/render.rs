//! Offline rendering of a project through the same `Player` the engine uses
//! (no audio device): for the golden render test and listening checks.

use crate::file::Project;

/// Output sample rate of offline renders (the engine's default).
pub const SAMPLE_RATE: u32 = 48_000;
/// Render buffer size (frames); the result doesn't depend on it.
const CHUNK: usize = 512;

/// Renders `secs` seconds of `p` from the start, mono, at [`SAMPLE_RATE`],
/// through the engine's `Mixdown` (console, studio tracks, Main gain).
pub fn render(p: &Project, secs: f64) -> Vec<f32> {
    let heard = crate::tracks::audible(&p.arrangement, &p.mute, &p.solo);
    let (mut m, snap) = crate::tracks::offline_mixdown(p, SAMPLE_RATE, &heard, false);
    m.player.set_loop_end(p.loop_end_ticks as f64);
    m.player.play();
    let mut out = vec![0f32; (secs * SAMPLE_RATE as f64) as usize];
    for chunk in out.chunks_mut(CHUNK) {
        m.render(&snap, chunk);
    }
    out
}

/// FNV-1a over the samples' bit patterns: any change in any sample changes it.
pub fn hash(samples: &[f32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for s in samples {
        for b in s.to_bits().to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}
