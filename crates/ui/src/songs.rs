//! Generated projects: the checked-in test song (`docs/songs/scale_test.chipdaw`)
//! and the large projects used to measure snapshot scaling and the piano roll.

use std::sync::Arc;

use crate::file::Project;
use crate::notes::{self, Note, RowKind};
use crate::state::{Clip, PPQ, TICKS_PER_BAR};

const SIX: u32 = PPQ / 4;

fn clip(id: u32, lane: usize, bar: u32, bars: u32, name: &str, mut notes: Vec<Note>) -> Arc<Clip> {
    for (i, n) in notes.iter_mut().enumerate() {
        n.id = i as u32 + 1;
    }
    let mut next = notes.len() as u32 + 1;
    notes::resolve_mono(&mut notes, &[], &mut next);
    Arc::new(Clip::new(id, lane, bar * TICKS_PER_BAR, bars * TICKS_PER_BAR, name.into(), notes))
}

fn n(start: u32, len: u32, pitch: u8) -> Note {
    Note::new(0, start, len, pitch)
}

/// Up and back down a scale (`steps` in semitones from `root`), one note per
/// `len` ticks, filling `bars` bars.
fn scale(root: u8, steps: &[u8], len: u32, bars: u32) -> Vec<Note> {
    let mut seq: Vec<u8> = steps.iter().map(|s| root + s).collect();
    let down: Vec<u8> = seq.iter().rev().skip(1).take(seq.len().saturating_sub(2)).copied().collect();
    seq.extend(down);
    (0..bars * TICKS_PER_BAR / len).map(|i| n(i * len, len, seq[i as usize % seq.len()])).collect()
}

/// Broken chords: each chord's notes cycle for one bar.
fn arps(chords: &[[u8; 3]], len: u32, bars: u32) -> Vec<Note> {
    (0..bars * TICKS_PER_BAR / len)
        .map(|i| {
            let ch = chords[(i * len / TICKS_PER_BAR) as usize % chords.len()];
            n(i * len, len, ch[i as usize % 3])
        })
        .collect()
}

const MAJOR: [u8; 15] = [0, 2, 4, 5, 7, 9, 11, 12, 14, 16, 17, 19, 21, 23, 24];
const MINOR: [u8; 8] = [0, 2, 3, 5, 7, 8, 10, 12];

/// "Scales and arpeggios on every channel": not a demo, a test song. Two
/// 4-bar clips per lane (scales, then arpeggios or patterns), 150 BPM, 8 bars.
pub fn scale_test() -> Project {
    let mut p = Project::new_legacy();
    let mut clips = Vec::new();
    let mut id = 0;
    let mut add = |lane, bar, name: &str, notes| {
        id += 1;
        clips.push(clip(id, lane, bar, 4, name, notes));
    };
    // Pulse 1: two-octave C major in 16ths; per-note volumes swell 4 -> 15.
    let mut p1 = scale(60, &MAJOR, SIX, 4);
    let len = p1.len();
    for (i, x) in p1.iter_mut().enumerate() {
        x.volume = Some((4 + i * 11 / len) as u8);
    }
    add(0, 0, "P1 scale", p1);
    add(0, 4, "P1 arps", arps(&[[60, 64, 67], [65, 69, 72], [67, 71, 74], [60, 67, 72]], SIX, 4));
    // Pulse 2: the low end of its range (A1 is its lowest note) in 8ths.
    add(1, 0, "P2 low scale", scale(33, &MINOR, SIX * 2, 4));
    add(1, 4, "P2 arps", arps(&[[57, 60, 64], [53, 57, 60], [55, 59, 62], [52, 55, 59]], SIX, 4));
    // Triangle: bass scale in quarters, then root-fifth-octave.
    add(2, 0, "Tri scale", scale(36, &MAJOR[..8], PPQ, 4));
    add(2, 4, "Tri arps", arps(&[[36, 43, 48], [41, 48, 53], [43, 50, 55], [36, 43, 48]], SIX * 2, 4));
    // Noise: every period index, slow to fast, then a beat.
    let noise = RowKind::for_lane(3);
    let sweep: Vec<Note> = (0..64u32).map(|i| n(i * SIX, SIX, noise.pitch_of(i % 16))).collect();
    add(3, 0, "Noise sweep", sweep);
    let beat: Vec<Note> = (0..64u32)
        .map(|i| {
            let row = match i % 8 {
                0 => 2,
                4 => 9,
                _ => 14,
            };
            Note { volume: Some(if i % 2 == 0 { 12 } else { 6 }), ..n(i * SIX, SIX, noise.pitch_of(row)) }
        })
        .collect();
    add(3, 4, "Noise beat", beat);
    // DPCM: each sample in turn, then a pattern.
    let dpcm = RowKind::for_lane(4);
    add(4, 0, "DPCM each", (0..16u32).map(|i| n(i * PPQ, PPQ, dpcm.pitch_of(i % 4))).collect());
    let pat = [0, 2, 1, 0, 0, 2, 1, 3];
    add(4, 4, "DPCM groove", (0..32u32).map(|i| n(i * SIX * 2, SIX * 2, dpcm.pitch_of(pat[i as usize % 8]))).collect());
    // VRC6 pulses: chromatic up an octave and back; minor arpeggios.
    let chroma: Vec<u8> = (0..=12).collect();
    add(5, 0, "V1 chromatic", scale(72, &chroma, SIX, 4));
    add(5, 4, "V1 arps", arps(&[[69, 72, 76], [65, 69, 72], [64, 67, 71], [69, 72, 76]], SIX, 4));
    add(6, 0, "V2 minor", scale(57, &MINOR, SIX * 2, 4));
    add(6, 4, "V2 arps", arps(&[[45, 48, 52], [41, 45, 48], [43, 47, 50], [45, 48, 52]], SIX * 2, 4));
    // Saw: scale in 8ths with a volume ramp (volume = accumulator rate).
    let mut saw = scale(36, &MAJOR, SIX * 2, 4);
    let len = saw.len();
    for (i, x) in saw.iter_mut().enumerate() {
        x.volume = Some((15 - i * 12 / len) as u8);
    }
    add(7, 0, "Saw scale", saw);
    add(7, 4, "Saw arps", arps(&[[36, 43, 48], [33, 40, 45], [29, 36, 41], [31, 38, 43]], SIX, 4));
    p.arrangement.next_id = id + 1;
    p.arrangement.clips = clips;
    p.loop_end_ticks = 8 * TICKS_PER_BAR;
    p
}

/// A project with `total` notes spread over 8 lanes in 4-bar clips of
/// `per_clip` notes each (for snapshot scaling).
pub fn many_notes(total: usize, per_clip: usize) -> Project {
    let mut p = Project::new_legacy();
    let clips_n = total.div_ceil(per_clip);
    let step = 4 * TICKS_PER_BAR / per_clip as u32;
    let mut clips = Vec::with_capacity(clips_n);
    for i in 0..clips_n {
        let lane = i % 8;
        let bar = (i / 8) as u32 * 4;
        let count = per_clip.min(total - i * per_clip);
        let kind = RowKind::for_lane(lane);
        let (lo, hi) = kind.playable_rows();
        let notes = (0..count as u32)
            .map(|k| n(k * step, step.max(2) - 1, kind.pitch_of(lo + (k * 7 + i as u32) % (hi - lo + 1))))
            .collect();
        clips.push(clip(i as u32 + 1, lane, bar, 4, "bulk", notes));
    }
    p.arrangement.next_id = clips.len() as u32 + 1;
    p.arrangement.clips = clips;
    p
}

/// The piano-roll benchmark: one Pulse 1 clip of 42 bars with a 1-bar
/// "handle" note at C4 (dragged by the benchmark) and 2,000 short notes
/// between C3 and A#4, all visible at 32 px per bar.
pub fn piano_bench() -> Project {
    let mut p = Project::new_legacy();
    let mut notes = vec![n(0, TICKS_PER_BAR, 60)];
    let span = 41 * TICKS_PER_BAR;
    let step = span / 2000;
    notes.extend((0..2000u32).map(|i| n(TICKS_PER_BAR + i * step, step - 4, 48 + ((i * 5) % 23) as u8)));
    p.arrangement.clips = vec![clip(1, 0, 0, 42, "2000 notes", notes)];
    p.arrangement.next_id = 2;
    p.loop_end_ticks = 42 * TICKS_PER_BAR;
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_songs_are_monophonic_and_sized() {
        let s = scale_test();
        assert_eq!(s.arrangement.clips.len(), 16);
        for c in &s.arrangement.clips {
            assert!(notes::is_mono(&c.notes), "{}", c.name);
            assert!(!c.notes.is_empty());
            let kind = RowKind::for_lane(c.lane);
            for x in &c.notes {
                assert!(kind.playable_row(kind.row_of(x.pitch)), "{} pitch {}", c.name, x.pitch);
            }
        }
        let m = many_notes(10_000, 100);
        assert_eq!(m.arrangement.clips.iter().map(|c| c.notes.len()).sum::<usize>(), 10_000);
        let b = piano_bench();
        assert_eq!(b.arrangement.clips[0].notes.len(), 2001);
    }
}
