//! Offline render to a WAV file: the same chip emulation and driver the
//! engine runs, driven faster than real time on the UI thread's request.
//! 48 kHz, 16-bit, mono (the console's output is mono).

use std::io::Write;
use std::path::Path;

use chip::player::Player;

use crate::file::Project;
use crate::state::SoundChip;

pub const SAMPLE_RATE: u32 = 48_000;
/// Silence-detection tail after the song end, so release tails ring out.
const TAIL_SECONDS: f64 = 2.0;
const BLOCK: usize = 4096;

/// Renders the song from the start to its loop end (plus the tail).
pub fn render(p: &Project) -> Vec<f32> {
    let mut player = Player::new(SAMPLE_RATE as f64);
    player.set_bpm(p.bpm as f64);
    player.set_vrc6(p.chip == SoundChip::WithVrc6);
    player.synth.set_filter(p.filter);
    player.driver.params = p.params;
    player.driver.mute = p.mute;
    player.driver.solo = p.solo;
    let end = p.loop_end_ticks as f64;
    // Render past the end without wrapping.
    player.set_loop_end(1.0e12);
    let snap = p.arrangement.song_snapshot(0);
    let song = snap.view();
    let seconds = end / (crate::state::PPQ as f64) * 60.0 / p.bpm as f64;
    let frames = ((seconds + TAIL_SECONDS) * SAMPLE_RATE as f64) as usize;
    let mut out = vec![0f32; frames];
    player.play();
    let song_frames = (seconds * SAMPLE_RATE as f64) as usize;
    let mut done = 0;
    while done < frames {
        let n = BLOCK.min(frames - done);
        if done >= song_frames && player.playing() {
            // Song over: stop so held notes release and their tails play out.
            player.stop();
        }
        player.render(&song, &mut out[done..done + n]);
        done += n;
    }
    out
}

/// Writes 16-bit mono PCM.
pub fn write_wav(samples: &[f32], path: &Path) -> Result<(), String> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    b.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        b.extend_from_slice(&v.to_le_bytes());
    }
    let tmp = path.with_extension("wav.tmp");
    std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(&b))
        .map_err(|e| format!("Can't write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Can't save {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_renders_audio_and_a_valid_header() {
        let mut p = Project::new_empty();
        p.arrangement = crate::mock::demo_arrangement();
        p.loop_end_ticks = 4 * crate::state::TICKS_PER_BAR;
        let s = render(&p);
        assert!(s.iter().all(|x| x.is_finite()));
        let peak = s.iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.05, "peak {peak}");
        let dir = std::env::temp_dir().join("chipdaw-export-test.wav");
        write_wav(&s[..4800], &dir).unwrap();
        let bytes = std::fs::read(&dir).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + 9600);
        let _ = std::fs::remove_file(dir);
    }
}
