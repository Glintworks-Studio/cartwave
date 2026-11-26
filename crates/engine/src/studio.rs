//! Studio tracks on the engine side (BATCH4A_PROMPT.md §4, VISION.md "Tracks
//! are instruments"): each studio track has a **pool of voices**, built and
//! grown on the UI thread, and a voice allocator that runs on the audio
//! thread at every frame tick.
//!
//! - **Unlimited polyphony by default.** The UI sizes each pool from the
//!   track's clips (the most notes sounding at once, plus headroom) and sends
//!   a bigger rack whenever a pool would be too small (`Command::Rack`). The
//!   audio thread moves the sounding voices into the new rack (no allocation:
//!   voices are plain data) and returns the old one to the UI thread to free.
//! - **Voices setting.** A track's cap (`TrackCtl::cap`, 0 = unlimited) limits
//!   how many of its pool's voices it uses.
//! - **Stealing.** If every usable voice is busy (the pool hasn't grown yet, or
//!   the cap is reached), a new note takes the **quietest** voice: one in its
//!   release tail before a held one, the oldest among equals.
//! - **Mixing.** Every voice adds its channel's DAC curve, alone, times its
//!   track's gain, into one shared step buffer (`chip::voice`). Gains are
//!   de-zippered: a change ramps over the next audio block in 4 steps.
//!
//! Live controls (gain, audible, the channel controls, the cap) are atomics in
//! `TrackCtl`, written by the UI thread whenever it likes; meters go the other
//! way through the same struct.

use crate::song::StudioLane;
use chip::blep::Blep;
use chip::dmc::ROM_SIZE;
use chip::driver::{Note, ParamId, Song};
use chip::instrument::NONE;
use chip::samples::{SampleRef, SAMPLE_COUNT, USER_SLOTS};
use chip::voice::{Voice, VoiceKind};
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU16, AtomicU32, Ordering};
use std::sync::Arc;

/// Channel controls per track (the driver's lane row).
pub const PARAMS: usize = ParamId::ALL.len();

/// One studio track's live controls and meters, shared by the UI and audio
/// threads (lock-free).
#[derive(Debug)]
pub struct TrackCtl {
    /// Linear gain (f32 bits): the track's fader.
    gain: AtomicU32,
    /// Heard (mute/solo applied by the UI).
    audible: AtomicBool,
    /// Most voices the track uses (0 = the whole pool: unlimited).
    cap: AtomicU16,
    params: [AtomicI8; PARAMS],
    /// Peak level since the UI last took it (f32 bits, max-merged).
    level: AtomicU32,
    /// Note-ons so far (wrapping): the strip light.
    pub note_ons: AtomicU32,
    /// Voices sounding after the last tick, and notes that found no free
    /// voice and stole one (wrapping counters, for tests and the CPU meter).
    pub sounding: AtomicU16,
    pub steals: AtomicU32,
}

impl TrackCtl {
    pub fn new(gain: f32, audible: bool, cap: u16, params: [i8; PARAMS]) -> Self {
        Self {
            gain: AtomicU32::new(gain.to_bits()),
            audible: AtomicBool::new(audible),
            cap: AtomicU16::new(cap),
            params: params.map(AtomicI8::new),
            level: AtomicU32::new(0),
            note_ons: AtomicU32::new(0),
            sounding: AtomicU16::new(0),
            steals: AtomicU32::new(0),
        }
    }

    pub fn set_gain(&self, g: f32) {
        self.gain.store(g.max(0.0).to_bits(), Ordering::Relaxed);
    }
    pub fn gain(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
    }
    pub fn set_audible(&self, on: bool) {
        self.audible.store(on, Ordering::Relaxed);
    }
    pub fn audible(&self) -> bool {
        self.audible.load(Ordering::Relaxed)
    }
    pub fn set_cap(&self, n: u16) {
        self.cap.store(n, Ordering::Relaxed);
    }
    pub fn cap(&self) -> u16 {
        self.cap.load(Ordering::Relaxed)
    }
    pub fn set_param(&self, id: ParamId, v: i8) {
        self.params[id as usize].store(v, Ordering::Relaxed);
    }
    pub fn params(&self) -> [i8; PARAMS] {
        std::array::from_fn(|i| self.params[i].load(Ordering::Relaxed))
    }
    fn merge_level(&self, v: f32) {
        self.level.fetch_max(v.max(0.0).to_bits(), Ordering::Relaxed);
    }
    /// Takes (and clears) the peak since the last call.
    pub fn take_level(&self) -> f32 {
        f32::from_bits(self.level.swap(0, Ordering::Relaxed))
    }
}

/// One voice of a pool and what the allocator gave it this tick.
#[derive(Clone)]
pub struct Slot {
    pub voice: Voice,
    want: Option<Note>,
    /// Keeps its sounding note this tick.
    taken: bool,
    /// Was given a new note this tick (can't be stolen again this tick).
    given: bool,
}

#[derive(Clone)]
pub struct StudioTrack {
    /// The UI's track id: rack and song snapshot meet on it.
    pub id: u32,
    pub kind: VoiceKind,
    pub ctl: Arc<TrackCtl>,
    pub slots: Box<[Slot]>,
    /// The gain the voices have now (ramps to `ctl.gain()`).
    gain_now: f32,
    /// Fresh voices (not carried over from an older rack) still need the
    /// console's clock and samples.
    fresh: bool,
}

impl StudioTrack {
    /// A track with `voices` silent voices (UI thread).
    pub fn new(id: u32, kind: VoiceKind, ctl: Arc<TrackCtl>, voices: usize, samples: [SampleRef; SAMPLE_COUNT]) -> Self {
        let g = ctl.gain();
        let slots = (0..voices.max(1))
            .map(|_| {
                let mut voice = Voice::new(kind, samples, 0);
                let _ = voice.set_gain_silently(g as f64);
                Slot { voice, want: None, taken: false, given: false }
            })
            .collect();
        Self { id, kind, ctl, slots, gain_now: g, fresh: true }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
}

/// Every studio track, in the UI's order (the song snapshot's `studio` lanes
/// have the same order and ids).
#[derive(Clone, Default)]
pub struct Rack {
    pub tracks: Vec<StudioTrack>,
    /// The frame tick count (for "the oldest" when stealing).
    ticks: u64,
}

impl Rack {
    pub fn new(tracks: Vec<StudioTrack>) -> Self {
        Self { tracks, ticks: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Takes over `old`'s sounding state (audio thread, no allocation): a
    /// track with the same id keeps its voices (swapped in whole when the pool
    /// size is unchanged, else copied into the bigger pool); new voices start
    /// on the console's clock `cycle` with the current DPCM samples.
    pub fn adopt(&mut self, old: &mut Rack, cycle: u64, user: &[SampleRef; USER_SLOTS]) {
        self.ticks = old.ticks;
        for t in self.tracks.iter_mut() {
            if let Some(o) = old.tracks.iter_mut().find(|o| o.id == t.id && o.kind == t.kind) {
                if o.slots.len() == t.slots.len() {
                    std::mem::swap(&mut o.slots, &mut t.slots);
                } else {
                    let n = o.slots.len().min(t.slots.len());
                    t.slots[..n].clone_from_slice(&o.slots[..n]);
                    for s in t.slots[n..].iter_mut() {
                        s.voice.resync(cycle);
                        s.voice.set_user_samples(*user);
                    }
                }
                t.gain_now = o.gain_now;
                t.fresh = false;
            }
            if t.fresh {
                for s in t.slots.iter_mut() {
                    s.voice.resync(cycle);
                    s.voice.set_user_samples(*user);
                }
                t.fresh = false;
            }
        }
    }

    /// New imported DPCM samples (their bytes are already in the ROM).
    pub fn set_user_samples(&mut self, user: &[SampleRef; USER_SLOTS]) {
        for t in self.tracks.iter_mut() {
            for s in t.slots.iter_mut() {
                s.voice.set_user_samples(*user);
            }
        }
    }

    /// Runs every voice up to CPU cycle `cycle`.
    pub fn run(&mut self, cycle: u64, rom: &[u8; ROM_SIZE], blep: &mut Blep) {
        for t in self.tracks.iter_mut() {
            for s in t.slots.iter_mut() {
                s.voice.run(cycle, rom, blep);
            }
        }
    }

    /// One frame tick at `cycle`: every track's allocator, then its voices'
    /// driver code. `lanes` are the snapshot's studio lanes (same order).
    #[allow(clippy::too_many_arguments)]
    pub fn tick(&mut self, song: &Song, lanes: &[StudioLane], cycle: u64, prev: Option<f64>, pos: f64, playing: bool, rom: &[u8; ROM_SIZE], blep: &mut Blep) {
        self.ticks += 1;
        let tick = self.ticks;
        for (i, t) in self.tracks.iter_mut().enumerate() {
            for s in t.slots.iter_mut() {
                s.voice.run(cycle, rom, blep);
            }
            let lane = lanes.get(i).filter(|l| l.id == t.id);
            let audible = t.ctl.audible();
            let params = t.ctl.params();
            let cap = match t.ctl.cap() {
                0 => t.slots.len(),
                c => (c as usize).min(t.slots.len()),
            };
            let notes: &[Note] = match lane {
                Some(l) if playing && audible => &l.notes,
                _ => &[],
            };
            let max_len = lane.map_or(0, |l| l.max_len);
            let inst = lane.map_or(NONE, |l| l.inst);
            allocate(&mut t.slots[..cap], notes, max_len, prev, pos, &t.ctl);
            // Voices past the cap (lowered while sounding) are released.
            for s in t.slots[cap..].iter_mut() {
                s.want = None;
            }
            let mut sounding = 0u16;
            for s in t.slots.iter_mut() {
                // The note's clip instrument, else the track's.
                let base = match s.want {
                    Some(n) if n.inst != NONE => n.inst,
                    _ => inst,
                };
                s.voice.tick(song, s.want, base, audible, &params, tick, rom, blep);
                if !s.voice.free() {
                    sounding += 1;
                }
            }
            t.ctl.sounding.store(sounding, Ordering::Relaxed);
        }
    }

    /// The end of a rendered chunk: gains ramp (4 steps over the chunk), every
    /// voice runs to `end`, meters are published.
    pub fn chunk(&mut self, end: u64, rom: &[u8; ROM_SIZE], blep: &mut Blep) {
        const RAMP_STEPS: u64 = 4;
        for t in self.tracks.iter_mut() {
            let target = t.ctl.gain();
            let ramp = target != t.gain_now;
            let g0 = t.gain_now as f64;
            let mut level = 0f32;
            let mut ons = 0u32;
            for s in t.slots.iter_mut() {
                if ramp {
                    let c0 = s.voice.cycle.min(end);
                    for k in 1..=RAMP_STEPS {
                        let ck = c0 + (end - c0) * k / RAMP_STEPS;
                        s.voice.run(ck.saturating_sub(1).max(c0), rom, blep);
                        let g = g0 + (target as f64 - g0) * k as f64 / RAMP_STEPS as f64;
                        s.voice.set_gain(if k == RAMP_STEPS { target as f64 } else { g }, blep);
                    }
                }
                s.voice.run(end, rom, blep);
                level = level.max(s.voice.take_peak());
                ons = ons.wrapping_add(s.voice.note_ons);
            }
            t.gain_now = target;
            t.ctl.merge_level(level);
            t.ctl.note_ons.store(ons, Ordering::Relaxed);
        }
    }
}

/// The allocator for one track at one frame tick. `slots` are the usable
/// voices (the cap applied). A note sounds if it runs at `pos`, or started
/// since `prev` (a note shorter than a frame still gets one frame, as on a
/// locked track). Each voice keeps its note while that note sounds; every
/// other sounding note takes a free voice, else steals the quietest.
fn allocate(slots: &mut [Slot], notes: &[Note], max_len: u32, prev: Option<f64>, pos: f64, ctl: &TrackCtl) {
    for s in slots.iter_mut() {
        s.taken = false;
        s.given = false;
    }
    if slots.is_empty() {
        return;
    }
    // Candidates: notes starting in [pos - max_len, pos].
    let hi = notes.partition_point(|n| (n.start as f64) <= pos);
    // Running notes start no earlier than `pos - max_len`; fresh ones after `prev`.
    let lo_t = (pos - max_len as f64).min(prev.unwrap_or(pos));
    let lo = notes[..hi].partition_point(|n| (n.start as f64) < lo_t);
    let sounds = |n: &Note| pos < n.end as f64 || prev.is_some_and(|p| n.start as f64 > p);
    // 1. Voices whose note still sounds keep it.
    for s in slots.iter_mut() {
        s.want = None;
        if let Some((start, pitch)) = s.voice.held {
            if s.voice.releasing() {
                continue;
            }
            let from = notes[lo..hi].partition_point(|n| n.start < start) + lo;
            if let Some(n) = notes[from..hi].iter().take_while(|n| n.start == start).find(|n| n.pitch == pitch) {
                if sounds(n) {
                    s.want = Some(*n);
                    s.taken = true;
                }
            }
        }
    }
    // 2. The other sounding notes, oldest first.
    for n in notes[lo..hi].iter().filter(|n| sounds(n)) {
        if slots.iter().any(|s| (s.taken || s.given) && s.want.is_some_and(|w| w.start == n.start && w.pitch == n.pitch)) {
            continue;
        }
        // A free voice, else the quietest one not given a note this tick:
        // release tails first, then held notes; the oldest among equals.
        let free = slots.iter().position(|s| !s.taken && !s.given && s.voice.free());
        // Only a note that has just started steals: a note that lost its
        // voice to a newer one stays lost (it would steal it back next tick).
        let starting = prev.is_none_or(|p| n.start as f64 > p);
        let pick = free.or_else(|| {
            if !starting {
                return None;
            }
            slots
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.given)
                .min_by_key(|(_, s)| (s.taken, !s.voice.releasing(), s.voice.loudness(), s.voice.started_tick))
                .map(|(i, _)| i)
        });
        let Some(i) = pick else { continue };
        if free.is_none() {
            ctl.steals.fetch_add(1, Ordering::Relaxed);
        }
        // A stolen voice gets another note, so its driver retriggers.
        slots[i].want = Some(*n);
        slots[i].taken = false;
        slots[i].given = true;
    }
}

/// The most notes of `notes` (sorted by start) sounding at once: the pool
/// size the UI needs (before headroom).
pub fn max_polyphony(notes: &[Note]) -> usize {
    let mut ends: Vec<u32> = Vec::new();
    let mut best = 0;
    for n in notes {
        ends.retain(|&e| e > n.start);
        ends.push(n.end.max(n.start + 1));
        best = best.max(ends.len());
    }
    best
}

/// The pool size for a track whose clips need `need` voices at once:
/// headroom for release tails and edits (×1.5 + 4, at least 8), rounded up
/// to a multiple of 8, never more than `cap` when one is set.
pub fn pool_size(need: usize, cap: u16) -> usize {
    let n = ((need * 3).div_ceil(2) + 4).max(8).div_ceil(8) * 8;
    if cap > 0 {
        n.min(cap as usize).max(1)
    } else {
        n
    }
}
