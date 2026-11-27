//! Batch 4b §5.2: the gain-staging listening pack for Mika. Each file is a
//! sustained pulse chord (4 s at 150 BPM) on studio tracks (the scheme: one
//! voice at the console's level, unison trimmed by 1/√n, the studio bus's
//! safety limiter at −1 dBFS) and, where the console has the channels, the
//! same chord on locked tracks (the 2A03 + VRC6 mixer, as the hardware).
//!
//! usage: cargo run --release -p ui --example listening_b4b -- <out dir>
//! Writes the WAVs and `levels.md` (sample peak, true peak, integrated
//! loudness) there.

use std::sync::Arc;

use chip::voice::VoiceKind;
use ui::file::Project;
use ui::notes::{Note, NoteFx};
use ui::state::{Clip, PPQ, TICKS_PER_BAR};
use ui::theme::tokens::Rgba;

/// A spread chord, low to high (C major-ish over two and a half octaves).
const CHORD: [u8; 16] = [48, 52, 55, 59, 62, 64, 67, 71, 72, 74, 76, 79, 83, 84, 86, 88];
const BARS: u32 = 2;

fn note(id: u32, pitch: u8) -> Note {
    Note { id, start: 0, len: BARS * TICKS_PER_BAR, pitch, volume: Some(15), fx: NoteFx::default() }
}

/// `n` voices of the chord on one studio pulse track (unison `u`, detune `d`).
fn studio(n: usize, u: u8, d: u8) -> Project {
    let mut p = Project::new_empty();
    p.arrangement.lanes.clear();
    p.arrangement.instruments.lanes.clear();
    let lane = p.arrangement.insert_track(0, VoiceKind::Pulse, Rgba::hex(0x4FA8E0));
    p.arrangement.lanes[lane].unison = ui::state::Unison { voices: u, detune: d, drift: 0 };
    let notes: Vec<Note> = CHORD[..n].iter().enumerate().map(|(i, &pi)| note(i as u32 + 1, pi)).collect();
    p.arrangement.clips.push(Arc::new(Clip::new(1, lane, 0, BARS * TICKS_PER_BAR, "Chord".into(), notes)));
    p.loop_end_ticks = BARS * TICKS_PER_BAR;
    p
}

/// The chord's first `n` notes on the console's pulse-like channels (P1, P2,
/// VRC6 P1, P2, Saw), one note each: the hardware's own mix.
fn console(n: usize) -> Project {
    let mut p = Project::new_legacy();
    let chans = [0usize, 1, 5, 6, 7];
    for (k, &c) in chans.iter().take(n).enumerate() {
        let notes = vec![note(1, CHORD[k])];
        p.arrangement.clips.push(Arc::new(Clip::new(k as u32 + 1, c, 0, BARS * TICKS_PER_BAR, "Note".into(), notes)));
    }
    p.loop_end_ticks = BARS * TICKS_PER_BAR;
    p
}

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "docs/audio/batch4b".into()));
    std::fs::create_dir_all(&out).unwrap();
    let mut rows = Vec::new();
    let mut put = |name: &str, what: &str, p: &Project| {
        let (x, ch) = ui::export::render_channels(p);
        // The sustained part only (the release tail and silence after it out).
        let frames = (BARS as f64 * 4.0 * 60.0 / p.bpm as f64 * 48_000.0) as usize * ch as usize;
        let body = &x[..frames.min(x.len())];
        let (lufs, tp, peak) = ui::game_export::measure_ch(body, 48_000, ch).unwrap();
        ui::export::write_wav_ch(&x, ch, &out.join(format!("{name}.wav"))).unwrap();
        rows.push(format!("| `{name}.wav` | {what} | {peak:+.1} | {tp:+.1} | {lufs:.1} |"));
        let _ = PPQ;
    };
    for n in [1usize, 3, 8, 16] {
        put(&format!("studio_{n:02}_voices"), &format!("{n} voice{} on a studio track", if n == 1 { "" } else { "s" }), &studio(n, 1, 0));
    }
    put("studio_unison4_detune15", "1 note, unison 4, detune 15 cents (studio)", &studio(1, 4, 15));
    put("studio_03_voices_unison4_detune15", "3 voices, unison 4, detune 15 (12 voices, studio)", &studio(3, 4, 15));
    for n in [1usize, 3, 5] {
        put(&format!("console_{n:02}_voices"), &format!("{n} note{} on locked tracks (P1, P2, VRC6 P1, P2, Saw)", if n == 1 { "" } else { "s" }), &console(n));
    }
    let md = format!(
        "# Batch 4b listening pack: gain staging (§5.2)\n\nA {BARS}-bar sustained pulse chord at 150 BPM, 48 kHz 16-bit mono (every track centred). Studio tracks use the scheme of RUNLOG B4B-D9: one voice at the console's level, unison voices trimmed by 1/√n, and the studio bus's safety limiter at −1 dBFS. Console files play the same notes on locked tracks (the 2A03 + VRC6 mixer): the console has five pulse-like channels, so 8 and 16 voices have no console version, and unison doesn't exist there.\n\nMeasured on the sustained part (`game_export::measure_ch`: EBU R128 integrated loudness, true peak, sample peak).\n\n| File | What | Sample peak (dBFS) | True peak (dBTP) | Loudness (LUFS) |\n|---|---|---|---|---|\n{}\n\nWritten by `cargo run --release -p ui --example listening_b4b -- docs/audio/batch4b`.\n",
        rows.join("\n")
    );
    std::fs::write(out.join("levels.md"), &md).unwrap();
    println!("{md}");
}
