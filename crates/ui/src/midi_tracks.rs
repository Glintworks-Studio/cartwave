//! MIDI import as tracks (BATCH4B_PROMPT.md §6, VISION.md "Importing MIDI"):
//! a `.mid` dropped on the arrangement (or File ▸ Import ▸ MIDI File…) adds
//! **one track per MIDI part** to the song; nothing already there changes.
//!
//! - **The plan** (`plan`): each part's name, its note count, the voice type
//!   it gets (drums → Noise, a part named "DPCM"/"Samples" → DPCM, with two or
//!   more melodic parts the lowest → Triangle, the rest → Pulse) and what it
//!   would break under the active hardware profile (`conflicts`).
//! - **Where** (`Target`): on empty space, new tracks after the last one; onto
//!   a track, the first part's clip goes on that track and the other parts get
//!   new tracks right below it (as the reference does with a multi-part file,
//!   unverified in the manual: logged B4B-D17). The clips start at the drop
//!   position, snapped to the bar.
//! - **How** (`Mode`): **Studio** plays everything as written on studio tracks;
//!   **Fit** assigns the profile's free channels (locked tracks), folds notes
//!   into each channel's range and reduces chords to one note at a time, or,
//!   with `arp`, turns each chord into a fast arpeggio (one note per frame
//!   tick, the classic chip trick). Parts left without a free channel stay
//!   studio tracks, so nothing is lost; the report says so.
//! - **Asking** (VISION principle 6): under no profile nothing conflicts and
//!   the parts arrive as studio tracks; under a profile, if every part fits
//!   they arrive locked with no question; otherwise the caller asks (Import as
//!   studio tracks · Fit to hardware · Fit, chords as arpeggios · Cancel).

use std::sync::Arc;

use chip::voice::VoiceKind;

use crate::midi::MidiSong;
use crate::notes::{Note, NoteFx, RowKind};
use crate::state::{Arrangement, Clip, SoundChip, MAX_SONG_TICKS, PPQ, TICKS_PER_BAR};

/// (start, end, pitch, velocity) at 960 PPQ.
pub type Span = (u32, u32, u8, u8);

/// One part of the file, as a track to be.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedPart {
    pub name: String,
    pub kind: VoiceKind,
    pub notes: Vec<Span>,
    /// The most notes sounding at once.
    pub poly: usize,
    /// From the part's first bar to the end of its last note (ticks).
    pub first: u32,
    pub len: u32,
}

impl PlannedPart {
    pub fn count(&self) -> usize {
        self.notes.len()
    }
}

/// What a part would break under a profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conflict {
    /// No free channel of its type (the budget).
    NoChannel,
    /// Chords or overlaps: a channel plays one note at a time.
    Chords,
    /// Notes outside the channel's range.
    Range(usize),
}

impl Conflict {
    pub fn text(&self) -> String {
        match self {
            Conflict::NoChannel => "no free channel".into(),
            Conflict::Chords => "chords".into(),
            Conflict::Range(n) => format!("{n} notes out of range"),
        }
    }
}

/// Where the parts land.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// New tracks after the last one.
    End,
    /// The first part onto track `i`, the others on new tracks below it.
    Onto(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Studio,
    /// Fit to hardware; `arp`: chords become fast arpeggios.
    Fit { arp: bool },
}

fn max_poly(notes: &[Span]) -> usize {
    let mut ends: Vec<u32> = Vec::new();
    let mut best = 0;
    for n in notes {
        ends.retain(|&e| e > n.0);
        ends.push(n.1.max(n.0 + 1));
        best = best.max(ends.len());
    }
    best
}

/// The file's parts as tracks (file order; drums merged into one part).
pub fn plan(song: &MidiSong) -> Vec<PlannedPart> {
    let parts = song.parts();
    let name_of = |t: usize, ch: u8| song.track_names.get(t).filter(|n| !n.is_empty()).cloned().unwrap_or_else(|| format!("Track {} ch {}", t + 1, ch + 1));
    let is_dpcm = |t: usize| song.track_names.get(t).map(|n| n.to_lowercase()).is_some_and(|n| n.contains("dpcm") || n.contains("sample"));
    let mut out: Vec<(usize, PlannedPart)> = Vec::new();
    let mut drums: Vec<Span> = parts.iter().filter(|p| p.1 == 9).flat_map(|p| p.2.iter().map(|n| (n.0, n.1, crate::midi::drum_pitch(n.2), n.3))).collect();
    let mut order = 0;
    for p in parts.iter() {
        if p.1 == 9 {
            continue;
        }
        let kind = if is_dpcm(p.0) { VoiceKind::Dpcm } else { VoiceKind::Pulse };
        out.push((order, part(name_of(p.0, p.1), kind, p.2.clone())));
        order += 1;
    }
    // With two or more melodic parts, the lowest is the bass (triangle).
    let melodic: Vec<usize> = out.iter().enumerate().filter(|(_, p)| p.1.kind == VoiceKind::Pulse).map(|(i, _)| i).collect();
    if melodic.len() >= 2 {
        let avg = |p: &PlannedPart| p.notes.iter().map(|n| n.2 as f32).sum::<f32>() / p.notes.len().max(1) as f32;
        if let Some(&b) = melodic.iter().min_by(|a, b| avg(&out[**a].1).total_cmp(&avg(&out[**b].1))) {
            out[b].1.kind = VoiceKind::Triangle;
        }
    }
    let mut v: Vec<PlannedPart> = out.into_iter().map(|(_, p)| p).collect();
    if !drums.is_empty() {
        drums.sort_by_key(|n| (n.0, std::cmp::Reverse(n.2)));
        v.push(part("Drums".into(), VoiceKind::Noise, drums));
    }
    v.retain(|p| !p.notes.is_empty());
    v
}

fn part(name: String, kind: VoiceKind, mut notes: Vec<Span>) -> PlannedPart {
    notes.retain(|n| n.1 <= MAX_SONG_TICKS);
    notes.sort_by_key(|n| (n.0, n.2));
    let first = notes.first().map_or(0, |n| n.0 / TICKS_PER_BAR * TICKS_PER_BAR);
    let last = notes.iter().map(|n| n.1).max().unwrap_or(first);
    let len = (last.div_ceil(TICKS_PER_BAR) * TICKS_PER_BAR).saturating_sub(first).max(TICKS_PER_BAR);
    PlannedPart { name, kind, poly: max_poly(&notes), notes, first, len }
}

/// The playable range of console channel `c`'s rows (melodic channels).
fn range_of(c: usize) -> Option<(u8, u8)> {
    match RowKind::for_lane(c) {
        RowKind::Piano { play_lo, play_hi, .. } => Some((play_lo, play_hi)),
        _ => None,
    }
}

/// The channels each part would get under profile `p` (free ones of its
/// type, in part order), given the song's tracks: `None` = no free channel.
pub fn channels_for(parts: &[PlannedPart], a: &Arrangement, p: SoundChip) -> Vec<Option<usize>> {
    let mut taken: Vec<usize> = a.lanes.iter().filter_map(|l| l.lock).collect();
    parts
        .iter()
        .map(|part| {
            // A pulse part may also take a VRC6 pulse or the saw when the
            // profile has them.
            let mut cands: Vec<usize> = crate::tracks::channels_of(part.kind).to_vec();
            if part.kind == VoiceKind::Pulse {
                cands.extend([5, 6, 7]);
            }
            let c = cands.into_iter().find(|c| p.has_channel(*c) && !taken.contains(c));
            if let Some(c) = c {
                taken.push(c);
            }
            c
        })
        .collect()
}

/// What each part would break under profile `p` (empty: it fits). Under no
/// profile (studio only) nothing conflicts.
pub fn conflicts(parts: &[PlannedPart], a: &Arrangement, p: SoundChip) -> Vec<Vec<Conflict>> {
    if p == SoundChip::StudioOnly {
        return vec![Vec::new(); parts.len()];
    }
    let chans = channels_for(parts, a, p);
    parts
        .iter()
        .zip(chans)
        .map(|(part, c)| {
            let mut v = Vec::new();
            let Some(c) = c else { return vec![Conflict::NoChannel] };
            if part.poly > 1 {
                v.push(Conflict::Chords);
            }
            if let Some((lo, hi)) = range_of(c) {
                let n = part.notes.iter().filter(|n| n.2 < lo || n.2 > hi).count();
                if n > 0 {
                    v.push(Conflict::Range(n));
                }
            }
            v
        })
        .collect()
}

/// Whether anything conflicts (the dialog is shown only then).
pub fn any_conflict(c: &[Vec<Conflict>]) -> bool {
    c.iter().any(|v| !v.is_empty())
}

/// Ticks per frame tick at `bpm` (the arpeggio step: one note per frame).
pub fn ticks_per_frame(bpm: f32) -> u32 {
    let frames_per_sec = chip::tables::FRAME_HZ_NTSC;
    ((PPQ as f64 * bpm as f64 / 60.0 / frames_per_sec).round() as u32).max(1)
}

/// One note at a time: at a shared start the highest note wins (a melody's
/// top line; for the triangle bass the lowest), and a note ends where the
/// next begins.
pub fn reduce(notes: &[Span], keep_low: bool) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::with_capacity(notes.len());
    let mut i = 0;
    while i < notes.len() {
        let start = notes[i].0;
        let mut j = i;
        while j < notes.len() && notes[j].0 == start {
            j += 1;
        }
        let pick = notes[i..j].iter().copied().reduce(|a, b| if (keep_low && b.2 < a.2) || (!keep_low && b.2 > a.2) { b } else { a }).unwrap();
        if let Some(last) = out.last_mut() {
            if last.1 > start {
                last.1 = start;
            }
        }
        out.push(pick);
        i = j;
    }
    out.retain(|n| n.1 > n.0);
    out
}

/// Chords into fast arpeggios: while several notes sound, one plays at a
/// time, cycling through them low to high every `step` ticks (a frame tick).
pub fn arpeggiate(notes: &[Span], step: u32) -> Vec<Span> {
    let mut cuts: Vec<u32> = notes.iter().flat_map(|n| [n.0, n.1]).collect();
    cuts.sort_unstable();
    cuts.dedup();
    let mut out: Vec<Span> = Vec::new();
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mut held: Vec<Span> = notes.iter().filter(|n| n.0 <= a && n.1 >= b && n.1 > a).copied().collect();
        if held.is_empty() {
            continue;
        }
        held.sort_by_key(|n| n.2);
        held.dedup_by_key(|n| n.2);
        if held.len() == 1 {
            let n = held[0];
            // Continue the same note across a cut when nothing else changed.
            if let Some(last) = out.last_mut() {
                if last.1 == a && last.2 == n.2 && last.3 == n.3 {
                    last.1 = b;
                    continue;
                }
            }
            out.push((a, b, n.2, n.3));
            continue;
        }
        let mut t = a;
        let mut k = 0;
        while t < b {
            let n = held[k % held.len()];
            let e = (t + step).min(b);
            out.push((t, e, n.2, n.3));
            t = e;
            k += 1;
        }
    }
    out
}

/// Notes folded into `(lo, hi)` by octaves; returns how many moved.
fn fold(notes: &mut [Span], range: Option<(u8, u8)>) -> usize {
    let Some((lo, hi)) = range else { return 0 };
    let mut moved = 0;
    for n in notes.iter_mut() {
        let mut q = n.2;
        while q < lo && q <= 127 - 12 {
            q += 12;
        }
        while q > hi && q >= 12 {
            q -= 12;
        }
        moved += usize::from(q != n.2);
        n.2 = q;
    }
    moved
}

/// Adds the parts to `a` at tick `at` (snapped by the caller), `target` and
/// `mode` as above, under profile `p` at `bpm`. Returns the report and the
/// indices of the tracks that got clips. Every existing clip and track keeps
/// its content (new tracks shift the ones below them, as Insert Track does).
#[allow(clippy::too_many_arguments)]
pub fn import(a: &mut Arrangement, parts: &[PlannedPart], at: u32, target: Target, mode: Mode, p: SoundChip, bpm: f32, pick_colour: &mut dyn FnMut(VoiceKind) -> crate::theme::tokens::Rgba) -> (String, Vec<usize>) {
    let mut report = Vec::new();
    let mut lanes = Vec::new();
    let chans = match mode {
        Mode::Fit { .. } if p != SoundChip::StudioOnly => channels_for(parts, a, p),
        _ => vec![None; parts.len()],
    };
    // Under a profile with nothing conflicting the parts arrive locked; with
    // Studio they stay studio tracks.
    let mut next = match target {
        Target::End => a.lanes.len(),
        Target::Onto(i) => i.min(a.lanes.len()),
    };
    for (k, part) in parts.iter().enumerate() {
        let onto = matches!(target, Target::Onto(_)) && k == 0;
        let lane = if onto {
            next
        } else {
            let at_ix = match target {
                Target::End => a.lanes.len(),
                Target::Onto(_) => next + 1,
            };
            let i = a.insert_track(at_ix, part.kind, pick_colour(part.kind));
            a.lanes[i].name = part.name.chars().take(crate::user_presets::MAX_NAME).collect();
            i
        };
        next = lane;
        let mut notes = part.notes.clone();
        let mut how = String::new();
        let chan = chans.get(k).copied().flatten();
        let lane_kind = a.lanes[lane].voice;
        if let Mode::Fit { arp } = mode {
            // The channel this part plays on: its own (locked), or the type's
            // first for the range check of a studio leftover.
            let range = range_of(chan.unwrap_or(lane_kind.lane()));
            if part.poly > 1 {
                notes = if arp && part.kind != VoiceKind::Noise {
                    how.push_str("chords as arpeggios, ");
                    arpeggiate(&notes, ticks_per_frame(bpm))
                } else {
                    how.push_str("chords reduced, ");
                    reduce(&notes, part.kind == VoiceKind::Triangle)
                };
            }
            let moved = fold(&mut notes, range);
            if moved > 0 {
                how.push_str(&format!("{moved} notes folded into range, "));
            }
            match chan {
                Some(c) if (!onto || a.lanes[lane].lock.is_none()) && a.lanes[lane].voice == VoiceKind::of_lane(c) && a.channel_owner(c).is_none() => {
                    a.lock_track(lane, c);
                    how.push_str(&format!("locked to {}", crate::tracks::channel_name(c)));
                }
                None => how.push_str("no free channel: a studio track"),
                _ => {}
            }
        }
        let first = part.first;
        let ns: Vec<Note> = notes
            .iter()
            .enumerate()
            .map(|(i, n)| Note {
                id: i as u32 + 1,
                start: n.0.saturating_sub(first),
                len: (n.1 - n.0).max(1),
                pitch: n.2,
                volume: Some(((n.3 as u32 * 15 + 63) / 127).clamp(1, 15) as u8),
                fx: NoteFx::default(),
            })
            .collect();
        let start = at.saturating_add(first).min(MAX_SONG_TICKS.saturating_sub(part.len));
        let id = a.next_id;
        a.next_id += 1;
        a.clips.push(Arc::new(Clip::new(id, lane, start, part.len, part.name.clone(), ns)));
        lanes.push(lane);
        let how = how.trim_end_matches(", ").to_string();
        report.push(if how.is_empty() { format!("{} ({} notes)", part.name, part.count()) } else { format!("{} ({} notes, {how})", part.name, part.count()) });
    }
    (report.join("; "), lanes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::tokens::Rgba;

    fn p(name: &str, kind: VoiceKind, notes: Vec<Span>) -> PlannedPart {
        part(name.into(), kind, notes)
    }

    fn chordy() -> PlannedPart {
        // C-E-G for a beat, then D alone.
        p("Keys", VoiceKind::Pulse, vec![(0, 960, 60, 100), (0, 960, 64, 100), (0, 960, 67, 100), (960, 1920, 62, 90)])
    }

    #[test]
    fn studio_only_never_conflicts_and_a_profile_finds_chords_range_and_budget() {
        let a = Arrangement::new_project();
        let parts = vec![chordy(), p("Bass", VoiceKind::Triangle, vec![(0, 960, 10, 100)])];
        assert!(!any_conflict(&conflicts(&parts, &a, SoundChip::StudioOnly)));
        let mut locked = Arrangement::empty(); // every channel taken
        let c = conflicts(&parts, &locked, SoundChip::WithVrc6);
        assert_eq!(c[0], vec![Conflict::NoChannel]);
        locked.lanes.truncate(1); // only Pulse 1 taken
        let c = conflicts(&parts, &locked, SoundChip::Base2A03);
        assert_eq!(c[0], vec![Conflict::Chords]);
        assert!(matches!(c[1][..], [Conflict::Range(1)]), "{c:?}");
    }

    #[test]
    fn reduce_keeps_one_note_at_a_time() {
        let r = reduce(&chordy().notes, false);
        assert_eq!(r, vec![(0, 960, 67, 100), (960, 1920, 62, 90)]);
        assert_eq!(max_poly(&r), 1);
        let low = reduce(&chordy().notes, true);
        assert_eq!(low[0].2, 60);
    }

    #[test]
    fn arpeggios_cycle_the_chord_one_frame_at_a_time() {
        let step = ticks_per_frame(150.0);
        assert_eq!(step, 40, "960 PPQ at 150 BPM: 40 ticks per 60.1 Hz frame");
        let r = arpeggiate(&chordy().notes, step);
        assert_eq!(max_poly(&r), 1);
        assert_eq!(&r[..4].iter().map(|n| (n.0, n.2)).collect::<Vec<_>>(), &[(0, 60), (40, 64), (80, 67), (120, 60)]);
        assert_eq!(*r.last().unwrap(), (960, 1920, 62, 90), "a lone note stays whole");
        // The chord's time is covered exactly.
        assert_eq!(r.iter().filter(|n| n.0 < 960).map(|n| n.1 - n.0).sum::<u32>(), 960);
    }

    #[test]
    fn importing_adds_tracks_and_leaves_the_song() {
        let mut a = Arrangement::new_project();
        let before_tracks = a.lanes.len();
        let c0 = a.add_clip(0, 0, TICKS_PER_BAR);
        let before = a.clip(c0).unwrap().clone();
        let parts = vec![chordy(), p("Bass", VoiceKind::Triangle, vec![(0, 960, 40, 100)])];
        let mut col = |_| Rgba::hex(0x4FA8E0);
        let (rep, lanes) = import(&mut a, &parts, 2 * TICKS_PER_BAR, Target::End, Mode::Studio, SoundChip::StudioOnly, 150.0, &mut col);
        assert_eq!(a.lanes.len(), before_tracks + 2);
        assert_eq!(lanes, vec![before_tracks, before_tracks + 1]);
        assert_eq!(a.lanes[before_tracks].name, "Keys");
        assert!(a.lanes[before_tracks..].iter().all(|l| l.lock.is_none()), "studio tracks");
        let keys = a.clips.iter().find(|c| c.lane == before_tracks).unwrap();
        assert_eq!((keys.start_ticks, keys.notes.len()), (2 * TICKS_PER_BAR, 4), "as written, at the drop bar");
        assert_eq!(*a.clip(c0).unwrap(), before, "the old clip is untouched");
        assert!(rep.contains("Keys (4 notes)"));
    }

    #[test]
    fn onto_a_track_puts_the_first_part_there_and_the_rest_below() {
        let mut a = Arrangement::new_project();
        let n = a.lanes.len();
        let parts = vec![chordy(), p("Bass", VoiceKind::Triangle, vec![(0, 960, 40, 100)])];
        let mut col = |_| Rgba::hex(0x4FA8E0);
        let (_, lanes) = import(&mut a, &parts, 0, Target::Onto(1), Mode::Studio, SoundChip::StudioOnly, 150.0, &mut col);
        assert_eq!(lanes, vec![1, 2]);
        assert_eq!(a.lanes.len(), n + 1);
        assert_eq!(a.lanes[2].name, "Bass");
    }

    #[test]
    fn fit_locks_free_channels_and_makes_parts_playable() {
        let mut a = Arrangement::new_project(); // studio tracks only: every channel free
        let parts = vec![chordy(), p("Bass", VoiceKind::Triangle, vec![(0, 960, 10, 100)])];
        let mut col = |_| Rgba::hex(0x4FA8E0);
        let n = a.lanes.len();
        let (rep, lanes) = import(&mut a, &parts, 0, Target::End, Mode::Fit { arp: false }, SoundChip::Base2A03, 150.0, &mut col);
        assert_eq!((a.lanes[lanes[0]].lock, a.lanes[lanes[1]].lock), (Some(0), Some(2)), "{rep}");
        let keys = a.clips.iter().find(|c| c.lane == n).unwrap();
        assert_eq!(keys.notes.len(), 2, "the chord reduced");
        let bass = a.clips.iter().find(|c| c.lane == n + 1).unwrap();
        let (lo, _) = range_of(2).unwrap();
        assert!(bass.notes[0].pitch >= lo, "folded into the triangle's range");
        // Arpeggios.
        let mut b = Arrangement::new_project();
        let (_, l2) = import(&mut b, &parts[..1], 0, Target::End, Mode::Fit { arp: true }, SoundChip::Base2A03, 150.0, &mut col);
        let arp = b.clips.iter().find(|c| c.lane == l2[0]).unwrap();
        assert!(arp.notes.len() > 20, "a beat of 40-tick steps plus the D");
    }
}
