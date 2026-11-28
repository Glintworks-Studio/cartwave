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
use chip::auto::{self as auto, AutoTarget, LaneAuto};
use chip::blep::Blep;
use chip::dmc::ROM_SIZE;
use chip::driver::{Note, ParamId, Song};
use chip::instrument::NONE;
use chip::samples::{SampleRef, SAMPLE_COUNT, USER_SLOTS};
use chip::voice::{pan_split, StepOut, Voice, VoiceKind};
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU16, AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

/// Channel controls per track (the driver's lane row).
pub const PARAMS: usize = ParamId::ALL.len();

/// The pan range: −`PAN_MAX` (left) … +`PAN_MAX` (right), the UI's.
pub const PAN_MAX: i8 = 50;

/// The pan law (Batch 4b §4): equal power, −3 dB at centre. The channel
/// gains are `√2·cos θ` and `√2·sin θ`, θ from 0 (left) to π/2 (right), so a
/// centred track is at unity in each channel (exactly 1.0: the mono build's
/// level) and a hard-panned one is +3 dB in its side and silent in the other;
/// `l² + r²` is 2 everywhere (constant power).
pub fn pan_gains(pan: i8) -> (f64, f64) {
    if pan == 0 {
        return (1.0, 1.0);
    }
    let x = pan.clamp(-PAN_MAX, PAN_MAX) as f64 / PAN_MAX as f64;
    let theta = (x + 1.0) * core::f64::consts::FRAC_PI_4;
    let k = core::f64::consts::SQRT_2;
    (k * chip::math::cos(theta), k * chip::math::sin(theta))
}

/// Unison (Batch 4b §5): 1–4 voices per note.
pub const UNISON_MAX: u8 = 4;
/// Detune: the unison voices spread over 0–50 cents (total width).
pub const DETUNE_MAX: u8 = 50;
/// Drift: a slow per-voice pitch wander of 0–10 cents.
pub const DRIFT_MAX: u8 = 10;

/// The unison gain trim: `n` voices of one note at `1/√n` each, so a unison
/// note keeps one voice's loudness (detuned voices add in power), and one
/// voice is exactly unity (the gain-staging scheme, RUNLOG B4B-D*).
pub fn unison_trim(n: u8) -> f64 {
    match n.clamp(1, UNISON_MAX) {
        1 => 1.0,
        n => 1.0 / (n as f64).sqrt(),
    }
}

/// Unison voice `k` of `n`'s detune in cents: spread evenly over `detune`
/// cents, centred on the note (0 for a single voice).
pub fn unison_cents(k: u8, n: u8, detune: u8) -> f64 {
    if n <= 1 || detune == 0 {
        return 0.0;
    }
    detune as f64 * (k as f64 / (n - 1) as f64 - 0.5)
}

/// Voice `slot`'s drift in cents at frame tick `tick`: two slow sines (about
/// 0.11 and 0.27 Hz at the 60.1 Hz frame rate) with per-voice phases, scaled
/// to ±`drift` cents. Pure arithmetic on the tick count (`chip::math::sin`),
/// so a render is the same on every machine.
pub fn drift_cents(slot: usize, tick: u64, drift: u8) -> f64 {
    if drift == 0 {
        return 0.0;
    }
    let t = tick as f64 / chip::tables::FRAME_HZ_NTSC;
    let h = (slot as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 11;
    let (p1, p2) = ((h & 0xFFFF) as f64 / 65536.0, ((h >> 16) & 0xFFFF) as f64 / 65536.0);
    let tau = core::f64::consts::TAU;
    let w = 0.6 * chip::math::sin(tau * (0.11 * t + p1)) + 0.4 * chip::math::sin(tau * (0.27 * t + p2));
    drift as f64 * w
}

/// One studio track's live controls and meters, shared by the UI and audio
/// threads (lock-free).
#[derive(Debug)]
pub struct TrackCtl {
    /// Linear gain (f32 bits): the track's fader.
    gain: AtomicU32,
    /// Pan, −`PAN_MAX` … +`PAN_MAX` (Batch 4b: heard, `pan_gains`).
    pan: AtomicI8,
    /// Unison voices per note (1–4), their detune spread and drift (cents).
    unison: AtomicU8,
    detune: AtomicU8,
    drift: AtomicU8,
    /// The track's output peak bound since the UI last took it (f32 bits, in
    /// the mix's units: 1.0 = 0 dBFS): the sum of its voices' peaks, after
    /// its gain and pan. The header's clip mark lights above 1.0.
    out_peak: AtomicU32,
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
            pan: AtomicI8::new(0),
            unison: AtomicU8::new(1),
            detune: AtomicU8::new(0),
            drift: AtomicU8::new(0),
            out_peak: AtomicU32::new(0),
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
    pub fn set_pan(&self, p: i8) {
        self.pan.store(p.clamp(-PAN_MAX, PAN_MAX), Ordering::Relaxed);
    }
    pub fn pan(&self) -> i8 {
        self.pan.load(Ordering::Relaxed)
    }
    /// Unison voices (1–4), detune (0–50 cents) and drift (0–10 cents).
    pub fn set_unison(&self, n: u8, detune: u8, drift: u8) {
        self.unison.store(n.clamp(1, UNISON_MAX), Ordering::Relaxed);
        self.detune.store(detune.min(DETUNE_MAX), Ordering::Relaxed);
        self.drift.store(drift.min(DRIFT_MAX), Ordering::Relaxed);
    }
    pub fn unison(&self) -> (u8, u8, u8) {
        (self.unison.load(Ordering::Relaxed).clamp(1, UNISON_MAX), self.detune.load(Ordering::Relaxed), self.drift.load(Ordering::Relaxed))
    }
    fn merge_out_peak(&self, v: f32) {
        self.out_peak.fetch_max(v.max(0.0).to_bits(), Ordering::Relaxed);
    }
    /// Takes (and clears) the output peak bound since the last call (1.0 =
    /// 0 dBFS).
    pub fn take_out_peak(&self) -> f32 {
        f32::from_bits(self.out_peak.swap(0, Ordering::Relaxed))
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

/// The studio bus's step buffers (Batch 4b §4): the centre (both channels),
/// left only and right only. With no panned track only the centre ever gets
/// a step, and the bus is the mono build's exactly.
#[derive(Clone)]
pub struct Steps {
    // Boxed: a step buffer is ~33 KB, and three inline overflowed the main
    // thread's stack where the engine is built.
    pub c: Box<Blep>,
    pub l: Box<Blep>,
    pub r: Box<Blep>,
    /// A side buffer has had a step (stereo output from now on).
    pub side: bool,
}

impl Steps {
    pub fn new(cpu_hz: f64, sample_rate: f64) -> Self {
        Self { c: Box::new(Blep::new(cpu_hz, sample_rate)), l: Box::new(Blep::new(cpu_hz, sample_rate)), r: Box::new(Blep::new(cpu_hz, sample_rate)), side: false }
    }
}

impl StepOut for Steps {
    fn step(&mut self, cycle: u64, d: [i32; 3]) {
        self.c.add(cycle, d[0]);
        if !self.side && (d[1] != 0 || d[2] != 0) {
            // The sides start now: until then they were never read, so they
            // take the centre's timing (they hold nothing yet).
            let c: &Blep = &self.c;
            self.l.sync_timing(c);
            self.r.sync_timing(c);
        }
        if d[1] != 0 {
            self.l.add(cycle, d[1]);
            self.side = true;
        }
        if d[2] != 0 {
            self.r.add(cycle, d[2]);
            self.side = true;
        }
    }
}

/// One voice of a pool and what the allocator gave it this tick.
#[derive(Clone)]
pub struct Slot {
    pub voice: Voice,
    want: Option<Note>,
    /// Which unison voice of its note this is (0 … unison − 1).
    uni: u8,
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
    /// The gain the voices have now (ramps to `ctl.gain()` × the unison
    /// trim).
    gain_now: f32,
    /// The channel gains `(l, r)` the voices have now (ramps to the pan's).
    pan_now: (f64, f64),
    /// Fresh voices (not carried over from an older rack) still need the
    /// console's clock and samples.
    fresh: bool,
    /// Batch 5: the fader or pan is automated (the chunk's block ramp then
    /// stays out of it), and the ramp in progress toward the automation's
    /// value (four fixed steps after a frame tick).
    automated: bool,
    ramp: Option<AutoRamp>,
}

/// A fader / pan ramp at fixed CPU cycles (Batch 5): steps `k = 1..=4` at
/// `start + k * RAMP_CYCLES`, whatever the audio blocks are, so a render is
/// the same with any buffer size.
#[derive(Clone, Copy, Debug)]
struct AutoRamp {
    start: u64,
    k: u64,
    from: (f32, (f64, f64)),
    to: (f32, (f64, f64)),
}

/// One step of an automation ramp: about 1 ms (a sixteenth of a frame), so a
/// change at a frame tick is complete in about 4 ms.
pub const RAMP_CYCLES: u64 = 1861;
const RAMP_STEPS: u64 = 4;

impl StudioTrack {
    /// Runs the voices up to `target`, applying the ramp steps that fall
    /// before it, each at its own cycle.
    fn run_to(&mut self, target: u64, rom: &[u8; ROM_SIZE], blep: &mut Steps) {
        while let Some(mut r) = self.ramp {
            let ck = r.start + r.k * RAMP_CYCLES;
            if ck > target {
                break;
            }
            let last = r.k >= RAMP_STEPS;
            let f = r.k as f64 / RAMP_STEPS as f64;
            let g = if last { r.to.0 as f64 } else { r.from.0 as f64 + (r.to.0 as f64 - r.from.0 as f64) * f };
            let (l, rr) = if last { r.to.1 } else { (r.from.1 .0 + (r.to.1 .0 - r.from.1 .0) * f, r.from.1 .1 + (r.to.1 .1 - r.from.1 .1) * f) };
            for s in self.slots.iter_mut() {
                s.voice.run(ck, rom, blep);
                s.voice.set_gain(g, blep);
                s.voice.set_pan(pan_split(l, rr), blep);
            }
            r.k += 1;
            self.ramp = if last { None } else { Some(r) };
        }
        for s in self.slots.iter_mut() {
            s.voice.run(target, rom, blep);
        }
    }
}

impl StudioTrack {
    /// A track with `voices` silent voices (UI thread).
    pub fn new(id: u32, kind: VoiceKind, ctl: Arc<TrackCtl>, voices: usize, samples: [SampleRef; SAMPLE_COUNT]) -> Self {
        let g = Self::target_gain(&ctl);
        let pan = pan_gains(ctl.pan());
        let slots = (0..voices.max(1))
            .map(|_| {
                let mut voice = Voice::new(kind, samples, 0);
                let _ = voice.set_gain_silently(g as f64);
                let _ = voice.set_pan_silently(pan_split(pan.0, pan.1));
                Slot { voice, want: None, uni: 0, taken: false, given: false }
            })
            .collect();
        Self { id, kind, ctl, slots, gain_now: g, pan_now: pan, fresh: true, automated: false, ramp: None }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// The voices' gain: the fader times the unison trim. Exactly the
    /// fader's gain with one voice per note.
    fn target_gain(ctl: &TrackCtl) -> f32 {
        match ctl.unison().0 {
            1 => ctl.gain(),
            n => (ctl.gain() as f64 * unison_trim(n)) as f32,
        }
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
                t.pan_now = o.pan_now;
                t.automated = o.automated;
                t.ramp = o.ramp;
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
    pub fn run(&mut self, cycle: u64, rom: &[u8; ROM_SIZE], blep: &mut Steps) {
        for t in self.tracks.iter_mut() {
            for s in t.slots.iter_mut() {
                s.voice.run(cycle, rom, blep);
            }
        }
    }

    /// One frame tick at `cycle`: every track's allocator, then its voices'
    /// driver code. `lanes` are the snapshot's studio lanes (same order).
    #[allow(clippy::too_many_arguments)]
    pub fn tick(&mut self, song: &Song, lanes: &[StudioLane], cycle: u64, prev: Option<f64>, pos: f64, playing: bool, rom: &[u8; ROM_SIZE], blep: &mut Steps) {
        self.ticks += 1;
        let tick = self.ticks;
        for (i, t) in self.tracks.iter_mut().enumerate() {
            t.run_to(cycle, rom, blep);
            let lane = lanes.get(i).filter(|l| l.id == t.id);
            let mut audible = t.ctl.audible();
            let params = t.ctl.params();
            // Batch 5: the track's automation at this frame tick.
            let ch = t.kind.lane();
            let envs = lane.map_or(&[][..], |l| &l.auto[..]);
            let la = if envs.is_empty() { LaneAuto::default() } else { LaneAuto::eval(envs, &lane.unwrap().auto_points, ch, pos) };
            let (mut g_auto, mut p_auto, mut uni_auto) = (None, None, [None; 3]);
            for e in envs {
                let pts = e.points(&lane.unwrap().auto_points);
                let Some(v) = auto::eval(pts, pos) else { continue };
                match e.target {
                    AutoTarget::Gain => g_auto = Some(auto::db_to_gain(v)),
                    AutoTarget::Pan => p_auto = Some(auto::quantise(v, -(PAN_MAX as i32), PAN_MAX as i32) as i8),
                    AutoTarget::Active => audible &= auto::quantise(v, 0, 1) != 0,
                    AutoTarget::Unison => uni_auto[0] = Some(auto::quantise(v, 1, UNISON_MAX as i32) as u8),
                    AutoTarget::Detune => uni_auto[1] = Some(auto::quantise(v, 0, DETUNE_MAX as i32) as u8),
                    AutoTarget::Drift => uni_auto[2] = Some(auto::quantise(v, 0, DRIFT_MAX as i32) as u8),
                    _ => {}
                }
            }
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
            let (u0, d0, r0) = t.ctl.unison();
            let (unison, detune, drift) = (uni_auto[0].unwrap_or(u0), uni_auto[1].unwrap_or(d0), uni_auto[2].unwrap_or(r0));
            // The fader and pan: an envelope's value, ramped at fixed cycles.
            if g_auto.is_some() || p_auto.is_some() || t.automated {
                let g = g_auto.unwrap_or(t.ctl.gain());
                let g = if unison <= 1 { g } else { (g as f64 * unison_trim(unison)) as f32 };
                let p = pan_gains(p_auto.unwrap_or(t.ctl.pan()));
                if (g, p) != (t.gain_now, t.pan_now) {
                    t.ramp = Some(AutoRamp { start: cycle, k: 1, from: (t.gain_now, t.pan_now), to: (g, p) });
                    t.gain_now = g;
                    t.pan_now = p;
                }
                t.automated = g_auto.is_some() || p_auto.is_some();
            }
            allocate(&mut t.slots[..cap], notes, max_len, prev, pos, &t.ctl, unison);
            // Voices past the cap (lowered while sounding) are released.
            for s in t.slots[cap..].iter_mut() {
                s.want = None;
            }
            let mut sounding = 0u16;
            for (k, s) in t.slots.iter_mut().enumerate() {
                // Unison detune and drift (0.0 without them: the note's own
                // period, as before).
                let cents = unison_cents(s.uni, unison, detune) + drift_cents(k, tick, drift);
                s.voice.set_detune(cents / 100.0);
                if s.voice.driver.auto[ch] != la {
                    s.voice.driver.auto[ch] = la;
                }
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
    pub fn chunk(&mut self, end: u64, rom: &[u8; ROM_SIZE], blep: &mut Steps) {
        for t in self.tracks.iter_mut() {
            // An automation ramp (fixed cycles) first; an automated fader
            // or pan has no block ramp.
            if t.ramp.is_some() {
                t.run_to(end, rom, blep);
            }
            let (target, pan) = if t.automated || t.ramp.is_some() { (t.gain_now, t.pan_now) } else { (StudioTrack::target_gain(&t.ctl), pan_gains(t.ctl.pan())) };
            let ramp = target != t.gain_now || pan != t.pan_now;
            let (g0, p0) = (t.gain_now as f64, t.pan_now);
            let mut level = 0f32;
            let mut bound = 0f64;
            let mut ons = 0u32;
            for s in t.slots.iter_mut() {
                if ramp {
                    let c0 = s.voice.cycle.min(end);
                    for k in 1..=RAMP_STEPS {
                        let ck = c0 + (end - c0) * k / RAMP_STEPS;
                        s.voice.run(ck.saturating_sub(1).max(c0), rom, blep);
                        let f = k as f64 / RAMP_STEPS as f64;
                        let last = k == RAMP_STEPS;
                        let g = if last { target as f64 } else { g0 + (target as f64 - g0) * f };
                        let (l, r) = if last { pan } else { (p0.0 + (pan.0 - p0.0) * f, p0.1 + (pan.1 - p0.1) * f) };
                        s.voice.set_gain(g, blep);
                        s.voice.set_pan(pan_split(l, r), blep);
                    }
                }
                s.voice.run(end, rom, blep);
                bound += s.voice.peak_out();
                level = level.max(s.voice.take_peak());
                ons = ons.wrapping_add(s.voice.note_ons);
            }
            t.gain_now = target;
            t.pan_now = pan;
            t.ctl.merge_level(level);
            // The louder channel's share of the bound (the voices' gain is in
            // `peak_out`; the pan adds up to +3 dB on one side).
            t.ctl.merge_out_peak((bound * pan.0.max(pan.1)) as f32);
            t.ctl.note_ons.store(ons, Ordering::Relaxed);
        }
    }
}

/// The allocator for one track at one frame tick. `slots` are the usable
/// voices (the cap applied). A note sounds if it runs at `pos`, or started
/// since `prev` (a note shorter than a frame still gets one frame, as on a
/// locked track). Each voice keeps its note while that note sounds; every
/// other sounding note takes a free voice, else steals the quietest.
#[allow(clippy::too_many_arguments)]
fn allocate(slots: &mut [Slot], notes: &[Note], max_len: u32, prev: Option<f64>, pos: f64, ctl: &TrackCtl, unison: u8) {
    let unison = unison.clamp(1, UNISON_MAX);
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
                // A unison voice the setting no longer has is released.
                if sounds(n) && s.uni < unison {
                    s.want = Some(*n);
                    s.taken = true;
                }
            }
        }
    }
    // 2. The other sounding notes, oldest first: each wants `unison` voices
    // (copy k of its note; one voice per note without unison).
    for n in notes[lo..hi].iter().filter(|n| sounds(n)) {
        for k in 0..unison {
            give(slots, n, k, prev, ctl);
        }
    }
}

/// Gives unison voice `k` of note `n` a voice unless one already has it: a
/// free voice, else (for a note that has just started) the quietest one.
fn give(slots: &mut [Slot], n: &Note, k: u8, prev: Option<f64>, ctl: &TrackCtl) {
    if slots.iter().any(|s| (s.taken || s.given) && s.uni == k && s.want.is_some_and(|w| w.start == n.start && w.pitch == n.pitch)) {
        return;
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
    let Some(i) = pick else { return };
    if free.is_none() {
        ctl.steals.fetch_add(1, Ordering::Relaxed);
    }
    // A stolen voice gets another note, so its driver retriggers.
    slots[i].want = Some(*n);
    slots[i].uni = k;
    slots[i].taken = false;
    slots[i].given = true;
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
