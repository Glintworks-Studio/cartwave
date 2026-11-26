//! Offline rendering of a project through the same `Player` the engine uses
//! (no audio device): for the golden render test and listening checks.

use engine::chip::player::Player;

use crate::file::Project;
use crate::state::SoundChip;

/// Output sample rate of offline renders (the engine's default).
pub const SAMPLE_RATE: u32 = 48_000;
/// Render buffer size (frames); the result doesn't depend on it.
const CHUNK: usize = 512;

/// Renders `secs` seconds of `p` from the start, mono, at [`SAMPLE_RATE`].
pub fn render(p: &Project, secs: f64) -> Vec<f32> {
    let snap = p.arrangement.song_snapshot(1);
    let mut pl = Box::new(Player::new(SAMPLE_RATE as f64));
    pl.set_bpm(p.bpm as f64);
    pl.set_loop_end(p.loop_end_ticks as f64);
    pl.driver.params = p.params;
    pl.driver.mute = p.mute;
    pl.driver.solo = p.solo;
    pl.set_vrc6(p.chip == SoundChip::WithVrc6);
    pl.synth.set_filter(p.filter);
    // The track volumes (polish 2 §3): a post-chip gain per track.
    pl.synth.set_gains(p.arrangement.gains());
    pl.play();
    let mut out = vec![0f32; (secs * SAMPLE_RATE as f64) as usize];
    for chunk in out.chunks_mut(CHUNK) {
        pl.render(&snap.view(), chunk);
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
