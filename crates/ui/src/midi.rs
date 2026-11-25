//! Standard MIDI File import (format 0 and 1), written from the SMF spec.
//!
//! Every chip channel plays one note at a time, so a MIDI file is *arranged*
//! onto the lanes rather than copied:
//!
//! - Notes on MIDI channel 10 (GM drums) go to the noise lane, mapped onto
//!   the kit rows (kick, snare/toms, closed hat, open hat, cymbals).
//! - Every other (track, channel) pair is a *part*. Parts are ranked by note
//!   count; with two or more, the lowest-pitched main part becomes the triangle bass and
//!   the rest fill Pulse 1, Pulse 2, then VRC6 Pulse 1, Pulse 2 and Saw.
//! - Chords are reduced to their top note (the bass part keeps its lowest).
//! - Velocity becomes per-note volume (0..15).
//! - A track named "DPCM" or "Samples" goes to the DPCM lane (C2.. = rows).
//!
//! Parts that don't fit are reported, not silently dropped.

use std::sync::Arc;

use crate::notes::{Note, NoteFx};
use crate::state::{Arrangement, Clip, MAX_SONG_BARS, MAX_SONG_TICKS, PPQ, TICKS_PER_BAR};

/// A note in song ticks: (start, end, pitch, velocity).
type Span = (u32, u32, u8, u8);
/// (track index, MIDI channel, notes).
type Part = (usize, u8, Vec<Span>);

/// A note in file order (absolute ticks at the file's resolution).
#[derive(Clone, Copy, Debug)]
struct RawNote {
    start: u64,
    end: u64,
    pitch: u8,
    vel: u8,
}

#[derive(Debug, Default)]
pub struct MidiSong {
    pub bpm: Option<f32>,
    /// (track index, channel 0..16, notes in song ticks at 960 PPQ)
    parts: Vec<Part>,
    pub track_names: Vec<String>,
    /// Tempo events in the file. Only the first is used (one song tempo).
    pub tempo_events: usize,
}

pub struct Imported {
    pub arrangement: Arrangement,
    pub bpm: Option<f32>,
    pub loop_end: u32,
    /// Human summary: what went where, what was dropped.
    pub report: String,
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Result<u8, String> {
        let v = *self.b.get(self.i).ok_or("The MIDI file ends early.")?;
        self.i += 1;
        Ok(v)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.b.get(self.i..self.i + n).ok_or("The MIDI file ends early.")?;
        self.i += n;
        Ok(s)
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn var(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let b = self.u8()?;
            v = (v << 7) | (b & 0x7F) as u32;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err("Bad variable-length number in the MIDI file.".into())
    }
}

pub fn parse(bytes: &[u8]) -> Result<MidiSong, String> {
    let mut r = Reader { b: bytes, i: 0 };
    if r.take(4)? != b"MThd" {
        return Err("Not a MIDI file (no MThd header).".into());
    }
    let hlen = r.u32()? as usize;
    let format = r.u16()?;
    let ntracks = r.u16()?;
    let division = r.u16()?;
    r.take(hlen.saturating_sub(6))?;
    if format > 1 {
        return Err(format!("MIDI format {format} isn't supported (only 0 and 1)."));
    }
    if division & 0x8000 != 0 {
        return Err("SMPTE-timed MIDI files aren't supported.".into());
    }
    let ppq = division.max(1) as u64;
    let mut song = MidiSong::default();
    for t in 0..ntracks as usize {
        // Skip unknown chunks.
        let (id, len) = loop {
            let id = r.take(4)?;
            let len = r.u32()? as usize;
            if id == b"MTrk" {
                break (id, len);
            }
            r.take(len)?;
        };
        let _ = id;
        let end = r.i + len;
        let mut tick = 0u64;
        let mut status = 0u8;
        // Open notes per (channel, pitch): start, velocity.
        let mut open: std::collections::HashMap<(u8, u8), (u64, u8)> = Default::default();
        let mut by_ch: [Vec<RawNote>; 16] = Default::default();
        let mut name = String::new();
        while r.i < end {
            tick += r.var()? as u64;
            let mut b = r.u8()?;
            if b < 0x80 {
                // Running status.
                r.i -= 1;
                b = status;
            }
            match b {
                0xFF => {
                    let kind = r.u8()?;
                    let len = r.var()? as usize;
                    let data = r.take(len)?;
                    match kind {
                        0x51 if len == 3 => song.tempo_events += 1,
                        _ => {}
                    }
                    match kind {
                        0x51 if len == 3 && song.bpm.is_none() => {
                            let us = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32;
                            if us > 0 {
                                song.bpm = Some(60_000_000.0 / us as f32);
                            }
                        }
                        0x03 if name.is_empty() => name = String::from_utf8_lossy(data).trim().to_string(),
                        _ => {}
                    }
                }
                0xF0 | 0xF7 => {
                    let len = r.var()? as usize;
                    r.take(len)?;
                }
                _ if b >= 0x80 => {
                    status = b;
                    let ch = b & 0x0F;
                    match b & 0xF0 {
                        0x80 | 0x90 => {
                            let pitch = r.u8()? & 0x7F;
                            let vel = r.u8()? & 0x7F;
                            if let Some((s, v)) = open.remove(&(ch, pitch)) {
                                // A zero-length note (on and off on one tick, as drum
                                // triggers often are) still plays: one tick, one frame.
                                by_ch[ch as usize].push(RawNote { start: s, end: tick.max(s + 1), pitch, vel: v });
                            }
                            if b & 0xF0 == 0x90 && vel > 0 {
                                open.insert((ch, pitch), (tick, vel));
                            }
                        }
                        0xA0 | 0xB0 | 0xE0 => {
                            r.take(2)?;
                        }
                        0xC0 | 0xD0 => {
                            r.take(1)?;
                        }
                        _ => return Err("Unknown MIDI event.".into()),
                    }
                }
                _ => return Err("Unknown MIDI event.".into()),
            }
        }
        // Notes still held when the track ends stop there (a missing
        // note-off is a common export bug; dropping them would lose notes).
        for ((ch, pitch), (s, v)) in open.drain() {
            by_ch[ch as usize].push(RawNote { start: s, end: tick.max(s + 1), pitch, vel: v });
        }
        r.i = end;
        song.track_names.push(name);
        let scale = |t: u64| (t.saturating_mul(PPQ as u64) / ppq).min(u32::MAX as u64) as u32;
        for (ch, notes) in by_ch.into_iter().enumerate() {
            if !notes.is_empty() {
                let mut v: Vec<_> = notes.iter().map(|n| (scale(n.start), scale(n.end), n.pitch, n.vel)).collect();
                v.sort_by_key(|n| (n.0, std::cmp::Reverse(n.2)));
                song.parts.push((t, ch as u8, v));
            }
        }
    }
    Ok(song)
}

/// GM drum key -> noise kit row pitch (see the Standard Kit zones).
fn drum_pitch(key: u8) -> u8 {
    match key {
        35 | 36 => 40,                       // kick
        37..=40 => 56,                       // snare, clap
        41 | 43 | 45 | 47 | 48 | 50 => 48,   // toms
        42 | 44 | 54 | 69 | 70 => 72,        // closed hat, pedal, shakers
        46 => 84,                            // open hat
        49 | 51 | 52 | 53 | 55 | 57 | 59 => 92, // cymbals
        _ => 76,
    }
}

/// Reduces to one note at a time: the top note (or the bottom one for a bass).
fn monophonic(notes: &[Span], keep_low: bool) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::with_capacity(notes.len());
    for &n in notes {
        if let Some(last) = out.last_mut() {
            if n.0 < last.1 {
                let better = if keep_low { n.2 < last.2 } else { n.2 > last.2 };
                if n.0 == last.0 {
                    if better {
                        *last = n;
                    }
                    continue;
                }
                if !better {
                    continue;
                }
                last.1 = n.0;
            }
        }
        out.push(n);
    }
    out
}

/// Arranges a parsed file onto the lanes. `vrc6`: the expansion lanes may be used.
pub fn arrange(song: &MidiSong, vrc6: bool) -> Imported {
    let mut lanes: Vec<(usize, Vec<Span>, String)> = Vec::new();
    let mut report = Vec::new();
    let part_name = |t: usize, ch: u8| {
        let n = song.track_names.get(t).filter(|n| !n.is_empty()).cloned();
        n.unwrap_or_else(|| format!("Track {} ch {}", t + 1, ch + 1))
    };
    // Drums.
    let mut drums: Vec<Span> = song
        .parts
        .iter()
        .filter(|p| p.1 == 9)
        .flat_map(|p| p.2.iter().map(|n| (n.0, n.1, drum_pitch(n.2), n.3)))
        .collect();
    drums.sort_by_key(|n| (n.0, std::cmp::Reverse(n.2)));
    if !drums.is_empty() {
        // Drum hits are short; keep the lowest (kick) on a shared start.
        lanes.push((3, monophonic(&drums, true), "Drums".into()));
    }
    // A track named "DPCM" or "Samples" plays the DPCM lane: C2..G2 pick
    // the sample rows (Kick, Snare, Tom, Crash, User 1..4).
    let is_dpcm = |p: &Part| {
        let n = song.track_names.get(p.0).map(|n| n.to_lowercase()).unwrap_or_default();
        n.contains("dpcm") || n.contains("sample")
    };
    let dpcm: Vec<Span> = song.parts.iter().filter(|p| p.1 != 9 && is_dpcm(p)).flat_map(|p| p.2.iter().copied()).collect();
    if !dpcm.is_empty() {
        lanes.push((4, monophonic(&dpcm, true), "Samples".into()));
    }
    // Melodic parts, biggest first.
    let mut parts: Vec<&Part> = song.parts.iter().filter(|p| p.1 != 9 && !is_dpcm(p)).collect();
    parts.sort_by_key(|p| std::cmp::Reverse(p.2.len()));
    let mut slots: Vec<usize> = vec![0, 1];
    if vrc6 {
        slots.extend([5, 6, 7]);
    }
    let room = slots.len() + 1;
    let main: Vec<_> = parts.iter().take(room).copied().collect();
    let avg = |p: &Part| p.2.iter().map(|n| n.2 as f32).sum::<f32>() / p.2.len().max(1) as f32;
    // With two or more parts, the lowest one is the bass (triangle).
    let bass = (main.len() >= 2)
        .then(|| main.iter().copied().min_by(|a, b| avg(a).total_cmp(&avg(b))))
        .flatten();
    if let Some(b) = bass {
        lanes.push((2, monophonic(&b.2, true), part_name(b.0, b.1)));
    }
    let mut slot = slots.into_iter();
    // The rest keep the file order (the lead usually comes first).
    let mut rest = main.clone();
    rest.sort_by_key(|p| (p.0, p.1));
    for p in rest.iter().filter(|p| bass.is_none_or(|b| !std::ptr::eq(**p, b))) {
        if let Some(l) = slot.next() {
            lanes.push((l, monophonic(&p.2, false), part_name(p.0, p.1)));
        }
    }
    for p in parts.iter().skip(room) {
        report.push(format!("skipped {} ({} notes): no free channel", part_name(p.0, p.1), p.2.len()));
    }

    if song.tempo_events > 1 {
        report.push(format!("{} tempo changes ignored (the song keeps the first tempo)", song.tempo_events - 1));
    }
    let mut a = Arrangement::empty();
    let mut end = 0u32;
    for (_, notes, name) in lanes.iter_mut() {
        let before = notes.len();
        notes.retain(|n| n.1 <= MAX_SONG_TICKS);
        if notes.len() < before {
            report.push(format!("{name}: {} notes after bar {MAX_SONG_BARS} dropped", before - notes.len()));
        }
    }
    for (lane, notes, name) in lanes {
        if notes.is_empty() {
            continue;
        }
        // Fold melodic notes the channel can't play into its range by
        // octaves (the driver would skip them: silent notes).
        let range = match crate::notes::RowKind::for_lane(lane) {
            crate::notes::RowKind::Piano { play_lo, play_hi, .. } => Some((play_lo, play_hi)),
            _ => None,
        };
        let mut folded = 0;
        let fold = |p: u8, folded: &mut usize| -> u8 {
            let Some((lo, hi)) = range else { return p };
            let mut q = p;
            while q < lo && q <= 127 - 12 {
                q += 12;
            }
            while q > hi && q >= 12 {
                q -= 12;
            }
            *folded += usize::from(q != p);
            q
        };
        let first = notes[0].0 / TICKS_PER_BAR * TICKS_PER_BAR;
        let last = notes.iter().map(|n| n.1).max().unwrap_or(first);
        let clip_end = last.div_ceil(TICKS_PER_BAR) * TICKS_PER_BAR;
        end = end.max(clip_end);
        let ns: Vec<Note> = notes
            .iter()
            .enumerate()
            .map(|(i, n)| Note {
                id: i as u32 + 1,
                start: n.0 - first,
                len: (n.1 - n.0).max(1),
                pitch: fold(n.2, &mut folded),
                volume: Some(((n.3 as u32 * 15 + 63) / 127).clamp(1, 15) as u8),
                fx: NoteFx::default(),
            })
            .collect();
        let lane_name = crate::state::default_lanes()[lane].name;
        report.push(format!("{} → {} ({} notes)", name, lane_name, ns.len()));
        if folded > 0 {
            report.push(format!("{folded} notes moved by octaves into the {lane_name} range"));
        }
        let id = a.next_id;
        a.next_id += 1;
        a.clips.push(Arc::new(Clip::new(id, lane, first, clip_end - first, name, ns)));
    }
    Imported { arrangement: a, bpm: song.bpm, loop_end: end.max(TICKS_PER_BAR), report: report.join("; ") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smf(tracks: &[Vec<u8>]) -> Vec<u8> {
        let mut f = b"MThd".to_vec();
        f.extend(6u32.to_be_bytes());
        f.extend(1u16.to_be_bytes());
        f.extend((tracks.len() as u16).to_be_bytes());
        f.extend(480u16.to_be_bytes());
        for t in tracks {
            f.extend(b"MTrk");
            f.extend((t.len() as u32).to_be_bytes());
            f.extend(t);
        }
        f
    }

    #[test]
    fn chords_reduce_drums_map_and_tempo_reads() {
        // Tempo 500000 us (120 BPM); a C-E-G chord then a lone D (running status).
        let t0 = vec![0, 0xFF, 0x51, 3, 0x07, 0xA1, 0x20, 0, 0xFF, 0x2F, 0];
        let t1 = vec![
            0, 0x90, 60, 100, 0, 64, 100, 0, 67, 100, // chord at 0
            0x83, 0x60, 0x80, 60, 0, 0, 64, 0, 0, 67, 0, // off at 480
            0, 0x90, 62, 64, 0x83, 0x60, 62, 0, // D 480..960 (running status, vel 0 = off)
            0, 0xFF, 0x2F, 0,
        ];
        let t2 = vec![0, 0x99, 36, 120, 0x60, 0x89, 36, 0, 0, 0x99, 38, 100, 0x60, 0x89, 38, 0, 0, 0xFF, 0x2F, 0];
        let song = parse(&smf(&[t0, t1, t2])).unwrap();
        assert_eq!(song.bpm.map(|b| b.round()), Some(120.0));
        let imp = arrange(&song, true);
        let clips = &imp.arrangement.clips;
        let mel = clips.iter().find(|c| c.lane == 0).expect("a lone melodic part is the lead");
        assert_eq!(mel.notes.iter().map(|n| (n.start, n.len, n.pitch)).collect::<Vec<_>>(), [(0, 960, 67), (960, 960, 62)]);
        let drums = clips.iter().find(|c| c.lane == 3).unwrap();
        assert_eq!(drums.notes.iter().map(|n| n.pitch).collect::<Vec<_>>(), [40, 56]);
        assert_eq!(imp.loop_end, TICKS_PER_BAR);
    }
}
