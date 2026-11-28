//! Tracks as instruments, hardware profiles and locks (BATCH4A_PROMPT.md §4,
//! §5; VISION.md "The core model").
//!
//! A track is an instrument of one **voice type**. A **studio** track plays
//! through its own pool of voices (any number of tracks, unlimited polyphony,
//! fader and pan work). A **locked** track is bound to one real channel of
//! the project's hardware profile and plays through the full chip emulator
//! (its non-linear mixer, filters, frame-tick driver): one note at a time,
//! fader pinned at 0 dB and pan at centre.
//!
//! This module has the model's rules (names, budget, insert / delete /
//! duplicate / reorder, lock / unlock) and how tracks map onto the engine
//! (the console's eight lanes, the studio rack).

use crate::state::{db_to_gain, Arrangement, Clip, Lane, Profile, SoundChip};
use chip::voice::VoiceKind;
use engine::{ClipInput, SongSnapshot, StudioLane};
use std::sync::Arc;

/// The type's name ("Pulse", "VRC6 Pulse"…): new tracks are named after it.
pub fn kind_name(k: VoiceKind) -> &'static str {
    match k {
        VoiceKind::Pulse => "Pulse",
        VoiceKind::Triangle => "Triangle",
        VoiceKind::Noise => "Noise",
        VoiceKind::Dpcm => "DPCM",
        VoiceKind::Vrc6Pulse => "VRC6 Pulse",
        VoiceKind::Saw => "Saw",
    }
}

/// A studio badge: the voice type in capitals, no lock ("PULSE").
pub fn kind_badge(k: VoiceKind) -> &'static str {
    match k {
        VoiceKind::Pulse => "PULSE",
        VoiceKind::Triangle => "TRIANGLE",
        VoiceKind::Noise => "NOISE",
        VoiceKind::Dpcm => "DPCM",
        VoiceKind::Vrc6Pulse => "VRC6 PULSE",
        VoiceKind::Saw => "SAW",
    }
}

/// Short names for clip names ("PUL 12").
pub fn kind_short(k: VoiceKind) -> &'static str {
    match k {
        VoiceKind::Pulse => "PUL",
        VoiceKind::Triangle => "TRI",
        VoiceKind::Noise => "NOI",
        VoiceKind::Dpcm => "DMC",
        VoiceKind::Vrc6Pulse => "VP",
        VoiceKind::Saw => "SAW",
    }
}

/// A track's short name (clip names): its channel's when locked ("P1"),
/// its type's when not ("PUL").
pub fn short_for(l: &Lane) -> &'static str {
    match l.lock {
        Some(c) => crate::state::CHANNEL_SHORT[c.min(7)],
        None => kind_short(l.voice),
    }
}

/// A locked badge's channel ("2A03 P1", "VRC6 SAW"), drawn after the lock glyph.
pub fn channel_badge(c: usize) -> &'static str {
    ["2A03 P1", "2A03 P2", "2A03 TRI", "2A03 NOI", "2A03 DPCM", "VRC6 P1", "VRC6 P2", "VRC6 SAW"][c.min(7)]
}

/// A channel's full name ("2A03 Pulse 1"): the lock menu and hover help.
pub fn channel_name(c: usize) -> &'static str {
    ["2A03 Pulse 1", "2A03 Pulse 2", "2A03 Triangle", "2A03 Noise", "2A03 DPCM", "VRC6 Pulse 1", "VRC6 Pulse 2", "VRC6 Saw"][c.min(7)]
}

/// The console channels of type `k`.
pub fn channels_of(k: VoiceKind) -> &'static [usize] {
    match k {
        VoiceKind::Pulse => &[0, 1],
        VoiceKind::Triangle => &[2],
        VoiceKind::Noise => &[3],
        VoiceKind::Dpcm => &[4],
        VoiceKind::Vrc6Pulse => &[5, 6],
        VoiceKind::Saw => &[7],
    }
}

/// Hover help on a locked track's fader (§5.3).
pub fn locked_help(c: usize) -> String {
    let what = match c {
        2 => "the chip's own level (the triangle has no Volume register)",
        4 => "the chip's own level (the DPCM has no Volume register)",
        7 => "the chip's Rate register (Inspector > Rate)",
        _ => "the chip's Volume register (Inspector > Volume)",
    };
    format!("Locked to the {}: level comes from {what}", channel_name(c))
}

/// The output filter a profile starts with (UX.md §3): the console's own
/// stage, or none for a studio-only project.
pub fn default_filter(p: Profile) -> engine::FilterMode {
    match p {
        SoundChip::StudioOnly => engine::FilterMode::Off,
        p => engine::FilterMode::default_for(p.vrc6()),
    }
}

/// One row of the channel budget: how many locked tracks of a type, of how
/// many real channels the profile has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetRow {
    pub kind: VoiceKind,
    pub used: usize,
    pub avail: usize,
}

impl BudgetRow {
    pub fn over(&self) -> bool {
        self.used > self.avail
    }
}

/// The channel budget in VISION.md's form, one row per type (§5.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budget {
    pub rows: Vec<BudgetRow>,
    /// Locked tracks on a channel the profile lacks (each counts as over).
    pub stray: usize,
}

impl Budget {
    pub fn over(&self) -> bool {
        self.stray > 0 || self.rows.iter().any(|r| r.over())
    }

    fn label(k: VoiceKind, compact: bool) -> &'static str {
        match (k, compact) {
            (VoiceKind::Pulse, false) => "Pulse",
            (VoiceKind::Triangle, false) => "Triangle",
            (VoiceKind::Noise, false) => "Noise",
            (VoiceKind::Dpcm, false) => "DPCM",
            (VoiceKind::Vrc6Pulse, false) => "VRC6 Pulse",
            (VoiceKind::Saw, false) => "Saw",
            (VoiceKind::Pulse, true) => "P",
            (VoiceKind::Triangle, true) => "T",
            (VoiceKind::Noise, true) => "N",
            (VoiceKind::Dpcm, true) => "D",
            (VoiceKind::Vrc6Pulse, true) => "V",
            (VoiceKind::Saw, true) => "S",
        }
    }

    /// Each row as (text, over): "Pulse 2/2" (or "P 2/2" compact).
    pub fn parts(&self, compact: bool) -> Vec<(String, bool)> {
        self.rows.iter().map(|r| (format!("{} {}/{}", Self::label(r.kind, compact), r.used, r.avail), r.over())).collect()
    }

    /// `Pulse 2/2 · Triangle 1/1 · …` (`P 2/2 · T 1/1 · …` compact).
    pub fn text(&self, compact: bool) -> String {
        self.parts(compact).into_iter().map(|p| p.0).collect::<Vec<_>>().join(" · ")
    }
}

/// The project's channel budget under `p` (no rows for None: studio only).
pub fn budget(a: &Arrangement, p: Profile) -> Budget {
    let kinds: &[VoiceKind] = match p {
        SoundChip::StudioOnly => &[],
        SoundChip::Base2A03 => &VoiceKind::ALL[..4],
        SoundChip::WithVrc6 => &VoiceKind::ALL,
    };
    let rows = kinds
        .iter()
        .map(|&kind| BudgetRow {
            kind,
            used: a.lanes.iter().filter(|l| l.lock.is_some() && l.voice == kind).count(),
            avail: channels_of(kind).iter().filter(|&&c| p.has_channel(c)).count(),
        })
        .collect();
    let stray = a.lanes.iter().filter(|l| l.lock.is_some_and(|c| !p.has_channel(c))).count();
    Budget { rows, stray }
}

/// Track ids kept outside the document (mute, solo): sorted, unique.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TrackSet(pub Vec<u32>);

impl TrackSet {
    pub fn contains(&self, id: u32) -> bool {
        self.0.binary_search(&id).is_ok()
    }
    pub fn set(&mut self, id: u32, on: bool) {
        match (self.0.binary_search(&id), on) {
            (Err(i), true) => self.0.insert(i, id),
            (Ok(i), false) => {
                self.0.remove(i);
            }
            _ => {}
        }
    }
    pub fn toggle(&mut self, id: u32) {
        let on = !self.contains(id);
        self.set(id, on);
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Only ids of tracks that exist.
    pub fn retain_in(&mut self, a: &Arrangement) {
        self.0.retain(|id| a.lanes.iter().any(|l| l.id == *id));
    }
}

/// Whether each track is heard: any solo wins, else not muted.
pub fn audible(a: &Arrangement, mute: &TrackSet, solo: &TrackSet) -> Vec<bool> {
    let any_solo = a.lanes.iter().any(|l| solo.contains(l.id));
    a.lanes.iter().map(|l| if any_solo { solo.contains(l.id) } else { !mute.contains(l.id) }).collect()
}

/// The console's mute mask (bit per channel) for the locked tracks heard;
/// the console's own solo mask stays 0 (`audible` already applied it).
pub fn console_mute(a: &Arrangement, heard: &[bool]) -> u8 {
    let mut m = 0u8;
    for (l, &h) in a.lanes.iter().zip(heard) {
        if let (Some(c), false) = (l.lock, h) {
            m |= 1 << c;
        }
    }
    m
}

/// One studio track as the engine's rack needs it: id, type, pool size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RackSlot {
    pub id: u32,
    pub kind: VoiceKind,
    pub pool: usize,
}

impl Arrangement {
    /// File > New (Batch 4a): four studio tracks (Pulse 1, Pulse 2,
    /// Triangle, Noise) on the default band's presets and colours; no
    /// hardware profile. The legacy eight locked lanes are `empty()`.
    pub fn new_project() -> Self {
        let mut a = Arrangement::empty();
        let legacy = a.lanes.clone();
        a.lanes.clear();
        a.instruments.lanes.clear();
        for (n, &chan) in [0usize, 1, 2, 3].iter().enumerate() {
            let k = VoiceKind::of_lane(chan);
            // G15: a new project's Pulse tracks never get the legacy lime.
            let colour = if k == VoiceKind::Pulse { crate::theme::tokens::NEW_PULSE } else { legacy[chan].color };
            let at = a.insert_track(n, k, colour);
            a.lanes[at].params = crate::instruments::default_params().values[chan];
        }
        let clips = a.clips.clone();
        a.instruments.prune(&clips);
        a
    }

    /// The index of the track with `id`.
    pub fn track_index(&self, id: u32) -> Option<usize> {
        self.lanes.iter().position(|l| l.id == id)
    }

    /// The track locked to console channel `c`.
    pub fn channel_owner(&self, c: usize) -> Option<usize> {
        self.lanes.iter().position(|l| l.lock == Some(c))
    }

    /// The next free track id.
    pub fn new_track_id(&self) -> u32 {
        self.lanes.iter().map(|l| l.id).max().unwrap_or(0) + 1
    }

    /// "Pulse 2" for the second Pulse track: the type's name and the lowest
    /// number no track of the project uses.
    pub fn next_track_name(&self, k: VoiceKind) -> String {
        let base = kind_name(k);
        (1..)
            .map(|n| format!("{base} {n}"))
            .find(|n| !self.lanes.iter().any(|l| &l.name == n))
            .unwrap_or_else(|| base.to_string())
    }

    /// Insert Track: a new studio track of `k` at index `at` (below the
    /// selected track), with `color`. Clips below move down. Returns its index.
    pub fn insert_track(&mut self, at: usize, k: VoiceKind, color: crate::theme::tokens::Rgba) -> usize {
        let at = at.min(self.lanes.len());
        let id = self.new_track_id();
        let name = self.next_track_name(k);
        let mut t = Lane::studio(id, k, name, color);
        // The type's default preset, as the browser's first one for it.
        let inst = crate::instruments::LaneInstrument::named(crate::instruments::default_preset_for(k));
        if let Some(pr) = chip::presets::find(&inst.name) {
            for &(pid, v) in pr.params {
                let (lo, hi, _) = pid.range(k.lane());
                t.params[pid as usize] = (v as i32).clamp(lo, hi) as i8;
            }
        }
        self.shift_lanes(at, 1);
        self.lanes.insert(at, t);
        self.instruments.insert_lane(at, inst);
        at
    }

    /// Clips on tracks at or after `from` move by `d` tracks (+1: a track was
    /// inserted at `from`; -1: the track at `from - 1`… callers remove first).
    fn shift_lanes(&mut self, from: usize, d: isize) {
        for c in self.clips.iter_mut() {
            if c.lane >= from {
                let c = Arc::make_mut(c);
                c.lane = (c.lane as isize + d).max(0) as usize;
                c.rev = crate::state::Rev::next();
            }
        }
        self.insert.1 = if self.insert.1 >= from { (self.insert.1 as isize + d).max(0) as usize } else { self.insert.1 };
    }

    /// Delete Track: the track and its clips. The last track can't go.
    pub fn delete_track(&mut self, i: usize) -> bool {
        if i >= self.lanes.len() || self.lanes.len() <= 1 {
            return false;
        }
        self.clips.retain(|c| c.lane != i);
        self.lanes.remove(i);
        self.instruments.remove_lane(i);
        self.shift_lanes(i + 1, -1);
        let ids: Vec<u32> = self.clips.iter().map(|c| c.id).collect();
        self.selection.retain(|id| ids.contains(id));
        self.track_sel.retain(|&t| t != i);
        for t in self.track_sel.iter_mut() {
            if *t > i {
                *t -= 1;
            }
        }
        self.insert.1 = self.insert.1.min(self.lanes.len() - 1);
        true
    }

    /// Duplicate Track: a copy below, with copies of its clips; a copy of a
    /// locked track is a studio track (its channel is taken), sounding the
    /// same as an unlocked copy would. Returns the new index.
    pub fn duplicate_track(&mut self, i: usize) -> Option<usize> {
        let src = self.lanes.get(i)?.clone();
        let at = i + 1;
        let mut t = src.clone();
        t.id = self.new_track_id();
        t.name = self.copy_name(&src.name);
        if let Some((db, pan)) = t.before_lock.take() {
            (t.vol_db, t.pan) = (db, pan);
        }
        t.lock = None;
        t.short = short_for(&t);
        let inst = self.instruments.lane(i).cloned().unwrap_or_else(crate::instruments::LaneInstrument::plain);
        self.shift_lanes(at, 1);
        self.lanes.insert(at, t);
        self.instruments.insert_lane(at, inst);
        let originals: Vec<Clip> = self.clips.iter().filter(|c| c.lane == i).map(|c| (**c).clone()).collect();
        for mut c in originals {
            c.id = self.next_id;
            self.next_id += 1;
            c.lane = at;
            c.rev = crate::state::Rev::next();
            self.push_clip(c);
        }
        Some(at)
    }

    fn copy_name(&self, name: &str) -> String {
        (2..)
            .map(|n| format!("{name} {n}"))
            .find(|n| !self.lanes.iter().any(|l| &l.name == n))
            .unwrap_or_else(|| name.to_string())
    }

    /// Moves the track at `from` to index `to` (a header drag), its clips with
    /// it. Returns whether anything moved.
    pub fn move_track(&mut self, from: usize, to: usize) -> bool {
        let n = self.lanes.len();
        if from >= n || to >= n || from == to {
            return false;
        }
        // The old index of whatever ends up at each new index.
        let mut order: Vec<usize> = (0..n).collect();
        let x = order.remove(from);
        order.insert(to, x);
        let mut new_of = vec![0; n];
        for (new, &old) in order.iter().enumerate() {
            new_of[old] = new;
        }
        let lanes = std::mem::take(&mut self.lanes);
        let mut slots: Vec<Option<Lane>> = lanes.into_iter().map(Some).collect();
        self.lanes = order.iter().map(|&o| slots[o].take().expect("each once")).collect();
        self.instruments.reorder(&order);
        for c in self.clips.iter_mut() {
            let nl = new_of[c.lane.min(n - 1)];
            if nl != c.lane {
                let c = Arc::make_mut(c);
                c.lane = nl;
                c.rev = crate::state::Rev::next();
            }
        }
        self.track_sel = self.track_sel.iter().map(|&t| new_of[t.min(n - 1)]).collect();
        self.insert.1 = new_of[self.insert.1.min(n - 1)];
        true
    }

    /// Locks track `i` to console channel `c`: the fader goes to 0 dB and pan
    /// to centre (remembered for unlocking). The caller has checked the
    /// channel is free, of the track's type, and asked about the reset.
    pub fn lock_track(&mut self, i: usize, c: usize) {
        let Some(t) = self.lanes.get_mut(i) else { return };
        if t.lock.is_none() {
            t.before_lock = Some((t.vol_db, t.pan));
        }
        t.lock = Some(c);
        t.short = short_for(t);
        t.vol_db = 0.0;
        t.pan = 0;
        // A locked track is monophonic: overlapping notes are trimmed by the
        // engine's last-note rule; the clips themselves are left as they are
        // (the caller fits them first if the user chose to).
    }

    /// The Voice selector (PRESETS_PROMPT.md §4.2): studio track `i` plays
    /// voice `k` from now on. Its channel controls start at `k`'s defaults
    /// (the old ones mean other registers); its name, colour, clips, fader,
    /// pan, Voices and unison stay. The rack (its voice pool) is rebuilt by
    /// the UI thread when the shape changes. A locked track's voice is its
    /// channel's: `false`, nothing changes.
    pub fn set_voice(&mut self, i: usize, k: VoiceKind) -> bool {
        let Some(t) = self.lanes.get_mut(i) else { return false };
        if t.lock.is_some() || t.voice == k {
            return false;
        }
        t.voice = k;
        t.params = engine::Params::default().values[k.lane()];
        t.short = short_for(t);
        true
    }

    /// Unlocks track `i`: a studio track that sounds the same, with its
    /// fader and pan back where they were before locking.
    pub fn unlock_track(&mut self, i: usize) {
        let Some(t) = self.lanes.get_mut(i) else { return };
        if t.lock.take().is_some() {
            if let Some((db, pan)) = t.before_lock.take() {
                t.vol_db = db;
                t.pan = pan;
            }
            t.short = short_for(t);
        }
    }

    /// A legacy eight-lane arrangement (a MIDI file made into a song) fitted
    /// to profile `p`: tracks locked to channels the profile lacks become
    /// studio tracks if they are empty, or always under None (studio only),
    /// so nothing is silenced by a profile the user didn't pick for it.
    pub fn fit_to_profile(&mut self, p: Profile) {
        for i in 0..self.lanes.len() {
            if let Some(c) = self.lanes[i].lock {
                let empty = !self.clips.iter().any(|x| x.lane == i);
                if !p.has_channel(c) && (empty || p == SoundChip::StudioOnly) {
                    self.unlock_track(i);
                }
            }
        }
    }

    /// Whether a track's fader or pan would change on locking (§5.2 asks).
    pub fn lock_resets(&self, i: usize) -> bool {
        self.lanes.get(i).is_some_and(|t| t.lock.is_none() && (t.vol_db != 0.0 || t.pan != 0))
    }

    /// The console's channel controls (each locked track's into its channel).
    pub fn console_params(&self) -> engine::Params {
        let mut p = engine::Params::default();
        for l in &self.lanes {
            if let Some(c) = l.lock {
                p.values[c] = l.params;
            }
        }
        p
    }

    /// Each console channel's gain (its locked track's fader; pinned at 0 dB,
    /// so 1.0 unless a file says otherwise).
    pub fn gains(&self) -> [f32; 8] {
        let mut g = [1.0; 8];
        for l in &self.lanes {
            if let Some(c) = l.lock {
                g[c] = db_to_gain(l.vol_db);
            }
        }
        g
    }

    /// The studio rack's shape: each studio track's id, type and pool size
    /// (the most notes its clips sound at once plus headroom, within its
    /// Voices setting).
    pub fn rack_shape(&self) -> Vec<RackSlot> {
        // Runs on every content change (each frame of a note drag): nothing
        // to resolve or build without studio tracks (B4A-D26).
        if self.lanes.iter().all(|l| l.lock.is_some()) {
            return Vec::new();
        }
        let snap = self.studio_lanes(&self.resolved_clips(), &crate::instruments::bank(&self.instruments, None));
        self.lanes
            .iter()
            .filter(|l| l.lock.is_none())
            .map(|l| {
                // Unison voices come from the track's pool (and count against
                // its Voices cap): each note needs `unison` of them.
                let need = snap.iter().find(|s| s.id == l.id).map_or(0, |s| engine::studio::max_polyphony(&s.notes)) * l.unison.voices.max(1) as usize;
                RackSlot { id: l.id, kind: l.voice, pool: engine::studio::pool_size(need, l.voices) }
            })
            .collect()
    }

    /// Studio tracks' notes for the engine, in track order.
    /// `clips`: the resolved clips (the caller has them already: resolving
    /// clones the arrangement while an edit is in progress).
    fn studio_lanes(&self, clips: &[Arc<Clip>], b: &crate::instruments::Bank) -> Vec<StudioLane> {
        self.lanes
            .iter()
            .enumerate()
            .filter(|(_, l)| l.lock.is_none())
            .map(|(i, l)| {
                let notes: Vec<(u32, Vec<engine::NoteInput>)> = clips
                    .iter()
                    .filter(|c| c.lane == i)
                    .map(|c| (c.id, c.audible_notes().map(|n| (n.start, n.len, n.pitch, n.volume)).collect()))
                    .collect();
                let inputs = clips.iter().filter(|c| c.lane == i).zip(&notes).map(|(c, (_, n))| ClipInput {
                    lane: i,
                    start: c.start_ticks,
                    len: c.len_ticks,
                    inst: c.instrument.filter(|id| self.instruments.contains(*id)).map_or(chip::instrument::NONE, |id| b.of(id)),
                    notes: n,
                });
                StudioLane::build(l.id, b.of(self.instruments.lane_id(i)), inputs)
            })
            .collect()
    }

    /// Builds the engine's immutable snapshot (UI thread): locked tracks'
    /// clips on their channels, studio tracks' notes by id.
    pub fn song_snapshot(&self, version: u64) -> SongSnapshot {
        let clips = self.resolved_clips();
        let b = crate::instruments::bank(&self.instruments, self.audition.as_ref());
        let notes: Vec<Vec<engine::NoteInput>> = clips
            .iter()
            .map(|c| c.audible_notes().map(|n| (n.start, n.len, n.pitch, n.volume)).collect())
            .collect();
        let console = clips.iter().zip(&notes).filter_map(|(c, n)| {
            let chan = self.lanes.get(c.lane)?.lock?;
            Some(ClipInput {
                lane: chan,
                start: c.start_ticks,
                len: c.len_ticks,
                // Only a real override: lane-default clips follow the lane.
                inst: c.instrument.filter(|id| self.instruments.contains(*id)).map_or(chip::instrument::NONE, |id| b.of(id)),
                notes: n,
            })
        });
        let mut lane_inst = [chip::instrument::NONE; engine::LANES];
        for (i, l) in self.lanes.iter().enumerate() {
            if let Some(c) = l.lock {
                lane_inst[c] = b.of(self.instruments.lane_id(i));
            }
        }
        let studio = self.studio_lanes(&clips, &b);
        SongSnapshot::build(console, version)
            .with_instruments(b.bank, lane_inst, b.preview)
            .with_dpcm(crate::dpcm::bank(&self.samples))
            .with_studio(studio)
    }
}

/// How many channels the project's audio has (Batch 4b §4, RUNLOG B4B-D*):
/// 2 once a studio track is panned off centre, else 1. Locked tracks and
/// the console are always centred (the hardware is mono), so a project of
/// locked tracks only is mono. Exports follow it: a mono WAV (byte-identical
/// to before) or a stereo one, the mix and every stem alike.
pub fn channels(a: &Arrangement) -> u16 {
    if a.lanes.iter().any(|l| l.lock.is_none() && l.pan != 0) {
        2
    } else {
        1
    }
}

/// The engine's rack for `shape` (UI thread): one `TrackCtl` per track,
/// reused from `ctls` when the track already has one.
pub fn build_rack(shape: &[RackSlot], ctls: &mut Vec<(u32, Arc<engine::TrackCtl>)>, a: &Arrangement, heard: &[bool]) -> Box<engine::Rack> {
    let samples = chip::samples::build(&mut [0u8; chip::dmc::ROM_SIZE]);
    ctls.retain(|(id, _)| shape.iter().any(|s| s.id == *id));
    let tracks = shape
        .iter()
        .map(|s| {
            let i = a.track_index(s.id).unwrap_or(0);
            let l = &a.lanes[i];
            let ctl = match ctls.iter().find(|(id, _)| *id == s.id) {
                Some((_, c)) => c.clone(),
                None => {
                    let c = Arc::new(engine::TrackCtl::new(db_to_gain(l.vol_db), heard.get(i).copied().unwrap_or(true), l.voices, l.params));
                    c.set_pan(l.pan);
                    c.set_unison(l.unison.voices, l.unison.detune, l.unison.drift);
                    ctls.push((s.id, c.clone()));
                    c
                }
            };
            engine::StudioTrack::new(s.id, s.kind, ctl, s.pool, samples)
        })
        .collect();
    Box::new(engine::Rack::new(tracks))
}

/// Writes the live controls (gain, heard, Voices, channel controls) of every
/// studio track into its `TrackCtl`.
pub fn sync_ctls(ctls: &[(u32, Arc<engine::TrackCtl>)], a: &Arrangement, heard: &[bool], sent: &mut Vec<Option<CtlSent>>) {
    sent.resize(ctls.len(), None);
    for (k, (id, c)) in ctls.iter().enumerate() {
        let Some(i) = a.track_index(*id) else { continue };
        let l = &a.lanes[i];
        // This runs every tick: a control whose inputs haven't changed since
        // it was last sent is skipped (Batch 4c §2). A new rack's controls
        // are new `Arc`s, so they are always sent.
        let now = CtlSent {
            ctl: Arc::as_ptr(c) as usize,
            vol_db: l.vol_db,
            pan: l.pan,
            unison: (l.unison.voices, l.unison.detune, l.unison.drift),
            audible: heard.get(i).copied().unwrap_or(true),
            voices: l.voices,
            params: l.params,
        };
        if sent[k].as_ref() == Some(&now) {
            continue;
        }
        c.set_gain(db_to_gain(l.vol_db));
        c.set_pan(l.pan);
        c.set_unison(l.unison.voices, l.unison.detune, l.unison.drift);
        c.set_audible(now.audible);
        c.set_cap(l.voices);
        for (k, &v) in l.params.iter().enumerate() {
            c.set_param(chip::driver::ParamId::ALL[k], v);
        }
        sent[k] = Some(now);
    }
}

/// What [`sync_ctls`] last sent to one studio track's controls.
#[derive(Clone, Debug, PartialEq)]
pub struct CtlSent {
    ctl: usize,
    vol_db: f32,
    pan: i8,
    unison: (u8, u8, u8),
    audible: bool,
    voices: u16,
    params: crate::state::TrackParams,
}

/// The offline mixdown of a project (renders and every export): the
/// console set up from the locked tracks, a rack for the studio tracks,
/// `heard` applied (the project's mute and solo: `audible`), the project's
/// profile, filter, tempo and Main gain. `samples: false` leaves imported
/// samples out of the ROM (the golden render never loaded them).
pub fn offline_mixdown(p: &crate::file::Project, rate: u32, heard: &[bool], samples: bool) -> (Box<engine::Mixdown>, SongSnapshot) {
    let (a, profile, filter, bpm, master_db) = (&p.arrangement, p.chip, p.filter, p.bpm as f64, p.master_db);
    let mut m = Box::new(engine::Mixdown::new(rate as f64));
    m.player.set_bpm(bpm);
    m.player.set_vrc6(profile.vrc6());
    m.set_filter(filter);
    // The track volumes (polish 2 §3): a post-chip gain per locked track.
    m.player.synth.set_gains(a.gains());
    m.player.driver.params = a.console_params();
    m.player.driver.mute = console_mute(a, heard);
    m.player.driver.solo = 0;
    let snap = a.song_snapshot(0);
    // Imported samples into the ROM, as the engine does (the golden render
    // never loaded them: `samples: false` keeps its hashes).
    if samples {
        m.player.synth.rom[chip::samples::USER_BASE..].copy_from_slice(&snap.dpcm.data);
        m.set_user_samples(snap.dpcm.refs);
    }
    let shape = a.rack_shape();
    if !shape.is_empty() {
        let mut ctls = Vec::new();
        let rack = build_rack(&shape, &mut ctls, a, heard);
        m.adopt_rack(rack);
    }
    m.set_master(db_to_gain(master_db));
    (m, snap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::tokens::Rgba;

    fn studio_project() -> Arrangement {
        let mut a = Arrangement::empty();
        let c = Rgba::hex(0x7F86E0);
        a.insert_track(8, VoiceKind::Pulse, c);
        a
    }

    /// Batch 4c §2: `sync_ctls` runs every tick and sends a control only
    /// when its track's inputs changed (or the control is new).
    #[test]
    fn sync_ctls_sends_only_changes() {
        let mut a = studio_project();
        let id = a.lanes[8].id;
        let shape = a.rack_shape();
        let heard = vec![true; a.lanes.len()];
        let mut ctls = Vec::new();
        let _rack = build_rack(&shape, &mut ctls, &a, &heard);
        let c = ctls.iter().find(|(i, _)| *i == id).unwrap().1.clone();
        let mut sent = Vec::new();
        sync_ctls(&ctls, &a, &heard, &mut sent);
        assert!(sent.iter().all(|s| s.is_some()), "everything is sent once");
        let before = sent.clone();
        sync_ctls(&ctls, &a, &heard, &mut sent);
        assert_eq!(sent, before, "nothing changed: nothing sent");
        a.lanes[8].pan = -30;
        sync_ctls(&ctls, &a, &heard, &mut sent);
        assert_ne!(sent, before, "a pan change is sent");
        assert_eq!(c.pan(), -30);
        // A new control for the same track (a rebuilt rack) is always sent.
        let mut fresh = vec![(id, Arc::new(engine::TrackCtl::new(1.0, true, 0, a.lanes[8].params)))];
        std::mem::swap(&mut ctls, &mut fresh);
        sync_ctls(&ctls, &a, &heard, &mut sent);
        assert_eq!(ctls[0].1.pan(), -30);
    }

    #[test]
    fn names_number_by_type() {
        let mut a = Arrangement::empty();
        // The legacy layout has "Pulse 1" and "Pulse 2" (locked): the next is 3.
        assert_eq!(a.next_track_name(VoiceKind::Pulse), "Pulse 3");
        assert_eq!(a.next_track_name(VoiceKind::Saw), "Saw 1");
        let i = a.insert_track(1, VoiceKind::Pulse, Rgba::hex(0x123456));
        assert_eq!((i, a.lanes[1].name.as_str(), a.lanes[1].lock), (1, "Pulse 3", None));
        assert_eq!(a.lanes.len(), 9);
        assert_eq!(a.instruments.lanes.len(), 9);
    }

    #[test]
    fn insert_delete_move_keep_clips_on_their_tracks() {
        let mut a = Arrangement::empty();
        let c0 = a.add_clip(0, 0, 960);
        let c2 = a.add_clip(2, 0, 960);
        a.insert_track(1, VoiceKind::Noise, Rgba::hex(0x123456));
        assert_eq!(a.clip(c0).unwrap().lane, 0);
        assert_eq!(a.clip(c2).unwrap().lane, 3, "the triangle moved down one");
        let tri = a.lanes[3].id;
        assert!(a.move_track(3, 0));
        assert_eq!(a.lanes[0].id, tri);
        assert_eq!(a.clip(c2).unwrap().lane, 0, "its clip moved with it");
        assert_eq!(a.clip(c0).unwrap().lane, 1);
        assert!(a.delete_track(0));
        assert!(a.clip(c2).is_none(), "deleting a track deletes its clips");
        assert_eq!(a.clip(c0).unwrap().lane, 0);
        assert_eq!(a.instruments.lanes.len(), a.lanes.len());
    }

    #[test]
    fn duplicate_copies_clips_and_unlocks() {
        let mut a = Arrangement::empty();
        a.add_clip(0, 0, 960);
        let i = a.duplicate_track(0).unwrap();
        assert_eq!(i, 1);
        assert_eq!(a.lanes[1].name, "Pulse 1 2");
        assert_eq!(a.lanes[1].lock, None, "a copy of a locked track is a studio track");
        assert_eq!(a.clips.iter().filter(|c| c.lane == 1).count(), 1);
        assert_eq!(a.clips.iter().filter(|c| c.lane == 2).count(), 0, "Pulse 2's clips stayed");
    }

    #[test]
    fn lock_resets_and_unlock_restores_the_mix() {
        let mut a = studio_project();
        let i = 8;
        a.lanes[i].vol_db = -6.0;
        a.lanes[i].pan = 20;
        assert!(a.lock_resets(i));
        // Channel 0 is taken by the legacy Pulse 1: unlock it first.
        a.unlock_track(0);
        a.lock_track(i, 0);
        assert_eq!((a.lanes[i].vol_db, a.lanes[i].pan, a.lanes[i].lock), (0.0, 0, Some(0)));
        a.unlock_track(i);
        assert_eq!((a.lanes[i].vol_db, a.lanes[i].pan, a.lanes[i].lock), (-6.0, 20, None));
    }

    #[test]
    fn the_budget_counts_locked_tracks_per_type() {
        let mut a = Arrangement::empty();
        let b = budget(&a, SoundChip::WithVrc6);
        assert_eq!(b.text(false), "Pulse 2/2 · Triangle 1/1 · Noise 1/1 · DPCM 1/1 · VRC6 Pulse 2/2 · Saw 1/1");
        assert_eq!(b.text(true), "P 2/2 · T 1/1 · N 1/1 · D 1/1 · V 2/2 · S 1/1");
        assert!(!b.over());
        // NES: the VRC6 tracks are strays (over).
        let n = budget(&a, SoundChip::Base2A03);
        assert_eq!(n.rows.len(), 4);
        assert_eq!(n.stray, 3);
        assert!(n.over());
        // A third pulse track locked onto... nothing free: studio tracks don't count.
        a.insert_track(0, VoiceKind::Pulse, Rgba::hex(0x123456));
        assert_eq!(budget(&a, SoundChip::WithVrc6).rows[0].used, 2);
        assert!(budget(&a, SoundChip::StudioOnly).rows.is_empty());
    }

    #[test]
    fn mute_and_solo_follow_track_ids() {
        let a = Arrangement::empty();
        let mut mute = TrackSet::default();
        let mut solo = TrackSet::default();
        mute.set(a.lanes[1].id, true);
        let h = audible(&a, &mute, &solo);
        assert_eq!(h.iter().filter(|x| !**x).count(), 1);
        assert_eq!(console_mute(&a, &h), 0b10);
        solo.set(a.lanes[3].id, true);
        let h = audible(&a, &mute, &solo);
        assert_eq!(h, (0..8).map(|i| i == 3).collect::<Vec<_>>());
        assert_eq!(console_mute(&a, &h), !0b1000u8);
    }
}
