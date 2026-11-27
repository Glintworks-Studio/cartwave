//! Offline render to a WAV file: the same chip emulation and driver the
//! engine runs, driven faster than real time on the UI thread's request.
//! 48 kHz, 16-bit, mono (the console's output is mono).

use std::io::Write;
use std::path::Path;


use crate::file::Project;

pub const SAMPLE_RATE: u32 = 48_000;
/// Silence-detection tail after the song end, so release tails ring out.
const TAIL_SECONDS: f64 = 2.0;
const BLOCK: usize = 4096;

/// The longest render (song plus tail), so a damaged or hand-edited loop end
/// can't ask for hours of audio.
pub const MAX_SECONDS: f64 = 60.0 * 60.0;

/// Renders the song from the start to its loop end (plus the tail).
pub fn render(p: &Project) -> Vec<f32> {
    render_limited(p, MAX_SECONDS)
}

/// `render`, cut off after `max_seconds` (song and tail together).
pub fn render_limited(p: &Project, max_seconds: f64) -> Vec<f32> {
    let heard = crate::tracks::audible(&p.arrangement, &p.mute, &p.solo);
    let (mut m, snap) = crate::tracks::offline_mixdown(p, SAMPLE_RATE, &heard, true);
    let end = p.loop_end_ticks as f64;
    // Render past the end without wrapping.
    m.player.set_loop_end(1.0e12);
    let seconds = end / (crate::state::PPQ as f64) * 60.0 / p.bpm as f64;
    let frames = ((seconds + TAIL_SECONDS).min(max_seconds) * SAMPLE_RATE as f64) as usize;
    let mut out = vec![0f32; frames];
    m.player.play();
    let song_frames = (seconds * SAMPLE_RATE as f64) as usize;
    let mut done = 0;
    while done < frames {
        let n = BLOCK.min(frames - done);
        if done >= song_frames && m.player.playing() {
            // Song over: stop so held notes release and their tails play out.
            m.player.stop();
        }
        m.render(&snap, &mut out[done..done + n]);
        done += n;
    }
    out
}

/// `render` with both channels: (interleaved samples, channels). A project
/// with no panned studio track is mono (`tracks::channels`): then this is
/// `render` exactly, one channel.
pub fn render_channels(p: &Project) -> (Vec<f32>, u16) {
    let ch = crate::tracks::channels(&p.arrangement);
    if ch == 1 {
        return (render(p), 1);
    }
    let heard = crate::tracks::audible(&p.arrangement, &p.mute, &p.solo);
    let (mut m, snap) = crate::tracks::offline_mixdown(p, SAMPLE_RATE, &heard, true);
    m.player.set_loop_end(1.0e12);
    let seconds = p.loop_end_ticks as f64 / (crate::state::PPQ as f64) * 60.0 / p.bpm as f64;
    let frames = ((seconds + TAIL_SECONDS).min(MAX_SECONDS) * SAMPLE_RATE as f64) as usize;
    let song_frames = (seconds * SAMPLE_RATE as f64) as usize;
    let mut out = Vec::with_capacity(frames * 2);
    let (mut l, mut r) = (vec![0f32; BLOCK], vec![0f32; BLOCK]);
    m.player.play();
    let mut done = 0;
    while done < frames {
        let n = BLOCK.min(frames - done);
        if done >= song_frames && m.player.playing() {
            m.player.stop();
        }
        m.render_stereo(&snap, &mut l[..n], &mut r[..n]);
        out.extend(l[..n].iter().zip(&r[..n]).flat_map(|(a, b)| [*a, *b]));
        done += n;
    }
    (out, 2)
}

/// Writes 16-bit PCM, `channels` interleaved.
pub fn write_wav_ch(samples: &[f32], channels: u16, path: &Path) -> Result<(), String> {
    let b = crate::game_export::wav_bytes_ch(samples, SAMPLE_RATE, 16, channels, None);
    let tmp = path.with_extension("wav.tmp");
    std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(&b))
        .map_err(|e| format!("Can't write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Can't save {}: {e}", path.display()))
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
        let mut p = Project::new_legacy();
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

    #[test]
    fn an_imported_sample_plays_on_its_dpcm_row() {
        use crate::notes::{Note, NoteFx};
        use crate::state::{Clip, PPQ};
        let tone: Vec<f32> = (0..20_000).map(|i| (i as f32 * 0.03).sin()).collect();
        let (smp, _) = crate::dpcm::convert("tone", &tone, 44_100, chip::samples::USER_ROM).unwrap();
        let mut p = Project::new_empty();
        p.arrangement.samples = vec![None, Some(smp)];
        p.loop_end_ticks = crate::state::TICKS_PER_BAR;
        // A studio DPCM track (Batch 4a: its voices read the same ROM).
        let lane = p.arrangement.insert_track(4, chip::voice::VoiceKind::Dpcm, crate::theme::tokens::Rgba::hex(0xE39A42));
        let row = crate::notes::RowKind::for_lane(4).pitch_of(5); // "User 2"
        let n = Note { id: 1, start: 0, len: PPQ, pitch: row, volume: None, fx: NoteFx::default() };
        p.arrangement.clips.push(std::sync::Arc::new(Clip::new(1, lane, 0, PPQ * 4, "s".into(), vec![n])));
        let out = render(&p);
        let peak = out[..24_000].iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.02, "peak {peak}");
        // An empty slot stays silent.
        p.arrangement.samples = vec![Some(p.arrangement.samples[1].clone().unwrap())];
        let quiet = render(&p);
        let peak = quiet[..24_000].iter().fold(0f32, |m, x| m.max(x.abs()));
        assert!(peak < 0.02, "empty slot played: {peak}");
    }
}
