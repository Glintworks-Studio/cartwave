//! The sound driver: turns notes into register writes once per video frame,
//! like a game's music routine running in the NMI handler.
//!
//! - Runs on frame ticks (NTSC: every 29780.5 CPU cycles = 60.0988 Hz).
//! - Each tick, for each lane, it looks up the note under the song position
//!   and issues note-on / note-off / parameter writes. So note starts and ends
//!   are quantised to frame ticks (up to 16.6 ms), exactly as on hardware.
//! - Only controls the hardware has: see `ParamId` and `ENGINE.md`.
//! - No allocation: writes go into a fixed-size `Writes` buffer.

use crate::auto::{AutoEnv, AutoPoint, LaneAuto};
use crate::instrument::{duty_2a03, duty_vrc6, scale_volume, Frame, Instrument, MacroState, NONE};
use crate::pitch::{self, Osc};
use crate::samples::{sample_for_pitch, SampleRef, BUILTIN_COUNT, SAMPLE_COUNT, START_LEVEL, USER_SLOTS};

pub const LANES: usize = 8;
pub const LANE_P1: usize = 0;
pub const LANE_P2: usize = 1;
pub const LANE_TRI: usize = 2;
pub const LANE_NOISE: usize = 3;
pub const LANE_DMC: usize = 4;
pub const LANE_V1: usize = 5;
pub const LANE_V2: usize = 6;
pub const LANE_SAW: usize = 7;

/// A note in song ticks (960 per quarter note). `end` is exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note {
    pub start: u32,
    pub end: u32,
    pub pitch: u8,
    /// Per-note volume 0..=15 (`None`: the lane's Volume parameter). Pulse,
    /// noise and VRC6 pulse use it as V; the saw maps it to its accumulator
    /// rate (`saw_rate_for_volume`); triangle and DMC have no volume.
    pub volume: Option<u8>,
    /// The instrument (bank index) of the clip this note came from; `NONE`:
    /// the lane's. Read only at note-on: a held note keeps its instrument.
    pub inst: u8,
}

impl Note {
    pub const fn new(start: u32, end: u32, pitch: u8) -> Self {
        Self { start, end, pitch, volume: None, inst: NONE }
    }
}

/// The saw's per-note volume: accumulator rate `round(v * 42 / 15)`, so 15 is
/// the loudest rate that doesn't wrap (42) and 0 is silent.
pub const fn saw_rate_for_volume(v: u8) -> u8 {
    let v = if v > 15 { 15 } else { v };
    ((v as u32 * 42 + 7) / 15) as u8
}

/// The song as the driver sees it: per lane, notes sorted by `start` and not
/// overlapping (the monophonic rule is enforced before this point).
#[derive(Clone, Copy, Debug)]
pub struct Song<'a> {
    pub lanes: [&'a [Note]; LANES],
    /// The instrument bank.
    pub instruments: &'a [Instrument],
    /// Each lane's instrument (bank index; `NONE` = plain channel controls).
    pub lane_inst: [u8; LANES],
    /// Instrument for audition notes (`Driver::preview`); `NONE` = the lane's.
    pub preview_inst: u8,
    /// Batch 5: each console lane's automation envelopes (empty: none), and
    /// the points they index.
    pub auto: [&'a [AutoEnv]; LANES],
    pub auto_points: &'a [AutoPoint],
}

impl<'a> Song<'a> {
    pub const EMPTY: Song<'static> = Song { lanes: [&[]; LANES], instruments: &[], lane_inst: [NONE; LANES], preview_inst: NONE, auto: [&[]; LANES], auto_points: &[] };

    /// Notes only; every lane plays its plain channel controls.
    pub const fn plain(lanes: [&'a [Note]; LANES]) -> Self {
        Song { lanes, instruments: &[], lane_inst: [NONE; LANES], preview_inst: NONE, auto: [&[]; LANES], auto_points: &[] }
    }

    pub fn instrument(&self, idx: u8) -> &Instrument {
        self.instruments.get(idx as usize).unwrap_or(&Instrument::PLAIN)
    }

    /// The instrument and pitch that play `pitch` on `lane`: a kit picks its
    /// zone's instrument (and maybe a fixed pitch); a kit without a zone for
    /// the pitch plays it plain.
    pub fn resolve(&self, lane: usize, pitch: u8) -> (u8, u8) {
        self.resolve_with(self.lane_inst[lane], pitch)
    }

    /// Like `resolve`, starting from bank entry `idx`.
    pub fn resolve_with(&self, idx: u8, pitch: u8) -> (u8, u8) {
        let inst = self.instrument(idx);
        if !inst.is_kit() {
            return (idx, pitch);
        }
        match inst.zone(pitch) {
            Some(z) if !self.instrument(z.inst).is_kit() => {
                (z.inst, if z.fixed_pitch == NONE { pitch } else { z.fixed_pitch })
            }
            _ => (NONE, pitch),
        }
    }
}

/// Hardware envelope use for pulse and noise: the `L` and `C` bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EnvMode {
    /// C = 1: constant volume V.
    #[default]
    Constant,
    /// C = 0, L = 0: decays from 15 to 0 at V + 1 quarter frames per step.
    Decay,
    /// C = 0, L = 1: decays and loops.
    Loop,
}

/// Every per-channel control the hardware actually has. Which ones apply to
/// which lane: `ParamId::for_lane`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ParamId {
    /// Pulse: 0..=3 (12.5/25/50/75%). VRC6 pulse: 0..=7 (1..8 / 16), 8 = digitized (constant).
    Duty,
    /// Pulse, noise, VRC6 pulse: V, 0..=15 (the decay rate in envelope modes).
    Volume,
    /// Pulse, noise: 0 constant, 1 decay, 2 loop.
    Envelope,
    /// Pulse: signed shift -7..=7. 0 = off; > 0 pitch falls (adds), < 0 rises (negate).
    Sweep,
    /// Pulse: sweep divider period P, 0..=7.
    SweepRate,
    /// Noise: 0 long (32767-step), 1 short (93/31-step).
    NoiseMode,
    /// Noise: -1 = from the note, else period index 0..=15.
    NoisePeriod,
    /// DMC: -1 = from the note (C kick, C# snare, D tom, D# crash...), else 0..=3.
    Sample,
    /// DMC: rate index 0..=15. Saw: accumulator rate 0..=63 (> 42 wraps).
    Rate,
    /// DMC: loop flag 0/1.
    Loop,
}

impl ParamId {
    pub fn for_lane(lane: usize) -> &'static [ParamId] {
        use ParamId::*;
        match lane {
            LANE_P1 | LANE_P2 => &[Duty, Volume, Envelope, Sweep, SweepRate],
            LANE_TRI => &[],
            LANE_NOISE => &[NoiseMode, NoisePeriod, Volume, Envelope],
            LANE_DMC => &[Sample, Rate, Loop],
            LANE_V1 | LANE_V2 => &[Duty, Volume],
            _ => &[Rate],
        }
    }

    /// (min, max, default) for this parameter on `lane`.
    pub fn range(self, lane: usize) -> (i32, i32, i32) {
        use ParamId::*;
        match (self, lane) {
            (Duty, LANE_V1 | LANE_V2) => (0, 8, 7),
            (Duty, _) => (0, 3, 2),
            (Volume, _) => (0, 15, 12),
            (Envelope, LANE_NOISE) => (0, 2, 1),
            (Envelope, _) => (0, 2, 0),
            (Sweep, _) => (-7, 7, 0),
            (SweepRate, _) => (0, 7, 3),
            (NoiseMode, _) => (0, 1, 0),
            (NoisePeriod, _) => (-1, 15, -1),
            (Sample, _) => (-1, SAMPLE_COUNT as i32 - 1, -1),
            (Rate, LANE_SAW) => (0, 63, 42),
            (Rate, _) => (0, 15, 15),
            (Loop, _) => (0, 1, 0),
        }
    }

    pub const ALL: [ParamId; 10] = [
        ParamId::Duty,
        ParamId::Volume,
        ParamId::Envelope,
        ParamId::Sweep,
        ParamId::SweepRate,
        ParamId::NoiseMode,
        ParamId::NoisePeriod,
        ParamId::Sample,
        ParamId::Rate,
        ParamId::Loop,
    ];
}

/// Parameter values for every lane (unused entries are ignored).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    pub values: [[i8; ParamId::ALL.len()]; LANES],
}

impl Default for Params {
    fn default() -> Self {
        let mut values = [[0i8; ParamId::ALL.len()]; LANES];
        for (lane, row) in values.iter_mut().enumerate() {
            for p in ParamId::ALL {
                row[p as usize] = p.range(lane).2 as i8;
            }
        }
        Self { values }
    }
}

impl Params {
    pub fn get(&self, lane: usize, p: ParamId) -> i32 {
        self.values[lane][p as usize] as i32
    }

    pub fn set(&mut self, lane: usize, p: ParamId, v: i32) {
        let (lo, hi, _) = p.range(lane);
        self.values[lane][p as usize] = v.clamp(lo, hi) as i8;
    }
}

/// Register writes produced by one tick, applied in order.
#[derive(Clone)]
pub struct Writes {
    buf: [(u16, u8); 96],
    len: usize,
}

impl Default for Writes {
    fn default() -> Self {
        Self {
            buf: [(0, 0); 96],
            len: 0,
        }
    }
}

impl Writes {
    #[inline]
    pub fn push(&mut self, addr: u16, v: u8) {
        // 8 lanes x at most 6 writes + init fits; overflow would be a bug.
        debug_assert!(self.len < self.buf.len());
        if self.len < self.buf.len() {
            self.buf[self.len] = (addr, v);
            self.len += 1;
        }
    }

    pub fn as_slice(&self) -> &[(u16, u8)] {
        &self.buf[..self.len]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct LaneState {
    /// The note currently sounding (identified by start + pitch, so an edit
    /// elsewhere in the lane doesn't retrigger it).
    note: Option<(u32, u8)>,
    /// The sounding note's own volume (None: the lane parameter).
    vol: Option<u8>,
    /// Shadow of the side-effect-free control register last written.
    ctl: Option<u8>,
    /// Pulse: sweep register. Noise: period register.
    sweep: Option<u8>,
    /// Period last written.
    period: Option<u16>,
    /// Bank index of the sounding instrument.
    inst: u8,
    /// A copy of it, taken at note-on: the macros run from this copy, so a
    /// new snapshot (another bank layout, a clip switched to another
    /// instrument) never changes a note that is already sounding.
    instr: Instrument,
    /// Pitch actually played (a kit zone can fix it).
    play_pitch: u8,
    mac: MacroState,
}

#[derive(Clone)]
pub struct Driver {
    pub params: Params,
    /// Bit per lane.
    pub mute: u8,
    pub solo: u8,
    /// VRC6 present: drive lanes 5..8.
    pub vrc6: bool,
    lanes: [LaneState; LANES],
    samples: [SampleRef; SAMPLE_COUNT],
    initialised: bool,
    /// Note-ons per lane (wrapping), for the UI's channel lights.
    pub note_ons: [u32; LANES],
    /// Lanes with a note sounding after the last tick.
    pub sounding: u8,
    /// Frame ticks run so far.
    pub ticks: u64,
    /// Audition notes (the piano roll's Preview command), played while the
    /// transport is stopped, regardless of mute/solo. `start` is a serial
    /// number: a new serial retriggers even on the same pitch.
    pub preview: [Option<Note>; LANES],
    /// A fixed pitch offset per lane in semitones, added to the instrument's
    /// fine pitch: a studio voice's unison detune and drift (Batch 4b §5).
    /// 0.0 everywhere on the console, so its periods are unchanged.
    pub detune: [f64; LANES],
    /// Batch 5: each lane's automation for this frame (channel controls in
    /// place of `params`, macros in place of the instrument's): set by `tick`
    /// from the song for the console, by the rack for a studio voice.
    pub auto: [LaneAuto; LANES],
}

impl Driver {
    pub fn new(samples: [SampleRef; SAMPLE_COUNT]) -> Self {
        Self {
            params: Params::default(),
            mute: 0,
            solo: 0,
            vrc6: true,
            lanes: [LaneState::default(); LANES],
            samples,
            initialised: false,
            note_ons: [0; LANES],
            sounding: 0,
            ticks: 0,
            preview: [None; LANES],
            detune: [0.0; LANES],
            auto: [LaneAuto::default(); LANES],
        }
    }

    /// Channel control `p` of `lane` as it plays this frame: the automation's
    /// value where an envelope drives it, else the control's.
    #[inline]
    fn pv(&self, lane: usize, p: ParamId) -> i32 {
        let a = &self.auto[lane];
        if a.mask & (1 << p as u16) != 0 {
            a.vals[p as usize] as i32
        } else {
            self.params.get(lane, p)
        }
    }

    /// The value channel control `p` of `lane` plays with now (automation
    /// applied): for the UI's displays and tests.
    pub fn effective(&self, lane: usize, p: ParamId) -> i32 {
        self.pv(lane, p)
    }

    fn audible(&self, lane: usize) -> bool {
        let bit = 1u8 << lane;
        if self.solo != 0 {
            self.solo & bit != 0
        } else {
            self.mute & bit == 0
        }
    }

    /// The note to play at `pos` (song ticks): the last note starting at or
    /// before `pos` if it is still running, or if it started after `prev`
    /// (a note shorter than a frame still gets one frame, like a tracker row).
    /// `prev == None` (after a seek or on start) only chases running notes.
    pub fn note_at(notes: &[Note], prev: Option<f64>, pos: f64) -> Option<Note> {
        let i = notes.partition_point(|n| (n.start as f64) <= pos);
        if i == 0 {
            return None;
        }
        let n = notes[i - 1];
        let fresh = prev.is_some_and(|p| n.start as f64 > p);
        if pos < n.end as f64 || fresh {
            Some(n)
        } else {
            None
        }
    }

    /// Forgets what is sounding so the next tick re-triggers (after a seek or
    /// a song reload). Notes are silenced by `release_all` first.
    pub fn release_all(&mut self, w: &mut Writes) {
        for lane in 0..LANES {
            if self.lanes[lane].note.is_some() {
                self.note_off(lane, w);
            }
        }
    }

    /// One frame tick. `pos` is the song position (ticks) now, `prev` the one
    /// at the previous tick (`None` after a seek, a wrap or when starting).
    /// `playing == false` releases everything.
    pub fn tick(&mut self, song: &Song, prev: Option<f64>, pos: f64, playing: bool, w: &mut Writes) {
        self.ticks += 1;
        self.tick_init(w);
        // Batch 5: the console lanes' automation at this frame (quantised to
        // the registers' steps); nothing to do on a song without any.
        for lane in 0..LANES {
            let envs = song.auto[lane];
            if envs.is_empty() {
                if self.auto[lane] != LaneAuto::default() {
                    self.auto[lane] = LaneAuto::default();
                }
            } else {
                self.auto[lane] = LaneAuto::eval(envs, song.auto_points, lane, pos);
            }
        }
        let mut sounding = 0u8;
        for lane in 0..LANES {
            let vrc6_lane = lane >= LANE_V1;
            let chip_present = !vrc6_lane || self.vrc6;
            let audible = self.audible(lane) && chip_present;
            let (want, base) = if playing && audible {
                let n = Self::note_at(song.lanes[lane], prev, pos);
                // The note's clip instrument, else the lane's.
                let base = n.map_or(NONE, |n| n.inst);
                (n, if base != NONE { base } else { song.lane_inst[lane] })
            } else if !playing && chip_present {
                let base = if song.preview_inst != NONE { song.preview_inst } else { song.lane_inst[lane] };
                (self.preview[lane], base)
            } else {
                (None, NONE)
            };
            self.tick_lane(lane, song, want, base, audible, w);
            if self.lanes[lane].note.is_some() {
                sounding |= 1 << lane;
            }
        }
        self.sounding = sounding;
    }

    /// The power-up writes, once, before the first tick's lanes.
    pub fn tick_init(&mut self, w: &mut Writes) {
        if !self.initialised {
            self.initialised = true;
            w.push(0x4015, 0x0F);
            w.push(0x4017, 0x40); // 4-step, IRQ inhibit
            w.push(0x4001, 0x08);
            w.push(0x4005, 0x08);
            w.push(0x4008, 0x80);
            w.push(0x9003, 0x00);
        }
    }

    /// One lane's part of a frame tick: `want` is the note that should sound
    /// now (`None`: release), `base` the instrument it starts from, `audible`
    /// whether a release tail may play out. The console runs it for each of
    /// its lanes; a studio voice (`voice.rs`) runs it for its one lane, with
    /// the note its track's voice allocator gave it.
    pub fn tick_lane(&mut self, lane: usize, song: &Song, want: Option<Note>, base: u8, audible: bool, w: &mut Writes) {
        {
            // Resolve the instrument (a kit picks one by pitch).
            let want = want.and_then(|n| {
                let (inst, pitch) = song.resolve_with(base, n.pitch);
                self.playable(lane, pitch).then_some((n, inst, pitch))
            });
            let cur = self.lanes[lane].note;
            match want {
                Some((n, inst, pitch)) if cur != Some((n.start, n.pitch)) => {
                    self.lanes[lane].vol = n.volume;
                    self.lanes[lane].inst = inst;
                    self.lanes[lane].instr = *song.instrument(inst);
                    self.lanes[lane].play_pitch = pitch;
                    self.lanes[lane].mac = MacroState::note_on(&self.lanes[lane].instr);
                    self.note_on(lane, song, w);
                    self.lanes[lane].note = Some((n.start, n.pitch));
                    self.note_ons[lane] = self.note_ons[lane].wrapping_add(1);
                }
                Some((n, _, _)) => {
                    self.lanes[lane].vol = n.volume;
                    let inst = self.lanes[lane].instr;
                    self.lanes[lane].mac.advance(&inst);
                    self.sustain(lane, song, w);
                }
                None if cur.is_some() => {
                    let inst = self.lanes[lane].instr;
                    let st = &mut self.lanes[lane].mac;
                    // A release tail plays out unless the lane was silenced.
                    if audible && inst.has_release() && lane != LANE_DMC {
                        if st.released {
                            st.advance(&inst);
                        } else {
                            st.release(&inst);
                        }
                        if st.done(&inst) {
                            self.note_off(lane, w);
                        } else {
                            self.sustain(lane, song, w);
                        }
                    } else {
                        self.note_off(lane, w);
                    }
                }
                None => {}
            }
        }
    }

    /// The note sounding on `lane` (start tick, pitch), if any (a release
    /// tail counts as sounding).
    pub fn lane_note(&self, lane: usize) -> Option<(u32, u8)> {
        self.lanes[lane].note
    }

    /// Whether `lane`'s note is in its release tail.
    pub fn lane_releasing(&self, lane: usize) -> bool {
        self.lanes[lane].note.is_some() && self.lanes[lane].mac.released
    }

    fn osc(lane: usize) -> Option<Osc> {
        match lane {
            LANE_P1 | LANE_P2 => Some(Osc::Pulse),
            LANE_TRI => Some(Osc::Triangle),
            LANE_V1 | LANE_V2 => Some(Osc::Vrc6Pulse),
            LANE_SAW => Some(Osc::Saw),
            _ => None,
        }
    }

    fn playable(&self, lane: usize, pitch: u8) -> bool {
        if lane == LANE_DMC {
            return self.samples[self.dmc_sample(pitch)].present;
        }
        Self::osc(lane).is_none_or(|o| pitch::period_for(o, pitch).is_some())
    }

    /// The sample a DMC note plays: the Sample control, or the note's own.
    fn dmc_sample(&self, pitch: u8) -> usize {
        let sel = self.pv(LANE_DMC, ParamId::Sample);
        if sel < 0 {
            sample_for_pitch(pitch)
        } else {
            sel as usize
        }
    }

    /// Installs the imported samples (their bytes are already in the ROM).
    pub fn set_user_samples(&mut self, refs: [SampleRef; USER_SLOTS]) {
        self.samples[BUILTIN_COUNT..].copy_from_slice(&refs);
    }

    /// The lane's channel volume (its Volume control, 0..=15). It scales
    /// everything the lane plays, like a driver's channel volume column:
    /// out = note volume × volume macro × channel volume.
    fn level(&self, lane: usize) -> u8 {
        self.pv(lane, ParamId::Volume) as u8
    }

    /// The sounding note's own volume, or full (15).
    fn note_volume(&self, lane: usize) -> u8 {
        self.lanes[lane].vol.map_or(15, |v| v.min(15))
    }

    /// The 4-bit volume for this frame: note × macro × channel.
    fn out_volume(&self, lane: usize, f: &Frame, macro_vol: bool) -> u8 {
        let v = self.note_volume(lane);
        let v = if macro_vol { scale_volume(v, f.volume) } else { v };
        scale_volume(v, self.level(lane))
    }

    /// The saw's accumulator rate: the note's volume as a rate (15 = 42, the
    /// loudest that doesn't wrap), scaled by the Rate control (42 = as is;
    /// above 42 it overdrives into the wrap distortion).
    fn saw_rate(&self, lane: usize) -> u8 {
        let base = self.lanes[lane].vol.map_or(42, saw_rate_for_volume) as u32;
        let r = self.pv(lane, ParamId::Rate) as u32;
        ((base * r + 21) / 42).min(63) as u8
    }

    /// Envelope bits + V for pulse/noise. A volume macro needs constant volume.
    fn env_bits(&self, lane: usize, f: &Frame, macro_vol: bool) -> u8 {
        if macro_vol {
            return 0x30 | self.out_volume(lane, f, true);
        }
        // In the hardware decay modes V is the decay speed, not a level, so
        // the channel volume can't scale it: the note's volume, else the
        // control (shown as "Decay" in the UI) sets the speed.
        let decay = || self.lanes[lane].vol.map_or(self.level(lane), |v| v.min(15));
        match self.pv(lane, ParamId::Envelope) {
            0 => 0x30 | self.out_volume(lane, f, false), // halt length, constant volume
            1 => decay(),                                 // length runs (254 half frames), decay
            _ => 0x20 | decay(),                          // halt length + loop, decay
        }
    }

    fn pulse_duty(&self, lane: usize, f: &Frame) -> u8 {
        f.duty.map_or(self.pv(lane, ParamId::Duty) as u8, duty_2a03)
    }

    fn pulse_sweep(&self, lane: usize) -> u8 {
        let s = self.pv(lane, ParamId::Sweep);
        if s == 0 {
            // Negate set: the target can never overflow, so low notes aren't muted.
            0x08
        } else {
            let p = self.pv(lane, ParamId::SweepRate) as u8;
            0x80 | (p << 4) | if s < 0 { 0x08 } else { 0 } | s.unsigned_abs() as u8
        }
    }

    fn vrc6_ctl(&self, lane: usize, f: &Frame, macro_vol: bool) -> u8 {
        let v = self.out_volume(lane, f, macro_vol);
        match f.duty {
            Some(d) => (duty_vrc6(d) << 4) | v,
            None => {
                let d = self.pv(lane, ParamId::Duty);
                if d >= 8 {
                    0x80 | 0x70 | v
                } else {
                    ((d as u8) << 4) | v
                }
            }
        }
    }

    fn noise_period(&self, lane: usize, f: &Frame) -> u8 {
        let p = self.pv(LANE_NOISE, ParamId::NoisePeriod);
        let short = f.duty.map_or(self.pv(LANE_NOISE, ParamId::NoiseMode) != 0, |d| d != 0);
        let base = if p < 0 { pitch::noise_index_for(self.lanes[lane].play_pitch) as i32 } else { p };
        ((short as u8) << 7) | (base - f.arp).clamp(0, 15) as u8
    }

    /// The period for the current frame (pitch + arpeggio + fine pitch).
    fn period(&self, lane: usize, f: &Frame) -> Option<u16> {
        let osc = Self::osc(lane)?;
        let st = &self.lanes[lane];
        let detune = self.detune[lane];
        if f.arp == 0 && f.fine == 0.0 && detune == 0.0 {
            return pitch::period_for(osc, st.play_pitch);
        }
        // Without a detune the sum is exactly the old one (bit-identical).
        let fine = if detune == 0.0 { f.fine } else { f.fine + detune };
        pitch::period_for_f(osc, st.play_pitch as f64 + f.arp as f64 + fine)
    }

    /// Writes the period: low byte when it changed, high byte only when its
    /// bits changed (a high write resets the 2A03 pulse phase: a click).
    fn write_period(&mut self, lane: usize, t: u16, first: bool, w: &mut Writes) {
        const LEN: u8 = 1 << 3;
        let (lo_addr, hi_addr, hi_flags) = match lane {
            LANE_P1 | LANE_P2 => (0x4002 + 4 * lane as u16, 0x4003 + 4 * lane as u16, LEN),
            LANE_TRI => (0x400A, 0x400B, LEN),
            LANE_V1 => (0x9001, 0x9002, 0x80),
            LANE_V2 => (0xA001, 0xA002, 0x80),
            _ => (0xB001, 0xB002, 0x80),
        };
        let old = self.lanes[lane].period;
        if first || old.map(|o| o as u8) != Some(t as u8) {
            w.push(lo_addr, t as u8);
        }
        if first || old.map(|o| o >> 8) != Some(t >> 8) {
            w.push(hi_addr, hi_flags | (t >> 8) as u8);
        }
        self.lanes[lane].period = Some(t);
    }

    fn note_on(&mut self, lane: usize, song: &Song, w: &mut Writes) {
        self.lanes[lane].period = None;
        self.lanes[lane].ctl = None;
        self.lanes[lane].sweep = None;
        self.apply(lane, song, true, w);
    }

    fn sustain(&mut self, lane: usize, song: &Song, w: &mut Writes) {
        self.apply(lane, song, false, w);
    }

    /// Writes this frame's registers for `lane` (`first`: note-on).
    fn apply(&mut self, lane: usize, _song: &Song, first: bool, w: &mut Writes) {
        // Length index 1 = 254 half frames (2.1 s): only matters in decay mode,
        // where the envelope reaches 0 within 1 s anyway.
        const LEN: u8 = 1 << 3;
        let inst = self.lanes[lane].instr;
        let ovr = self.auto[lane].mac;
        let f = self.lanes[lane].mac.frame_with(&inst, &ovr);
        let macro_vol = !inst.volume.is_empty() || ovr.volume.is_some() || ovr.gate.is_some();
        let st = self.lanes[lane];
        match lane {
            LANE_P1 | LANE_P2 => {
                let base = 0x4000 + 4 * lane as u16;
                let Some(t) = self.period(lane, &f) else { return };
                let ctl = (self.pulse_duty(lane, &f) << 6) | self.env_bits(lane, &f, macro_vol);
                if first || st.ctl != Some(ctl) {
                    w.push(base, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
                let sweep = self.pulse_sweep(lane);
                if first || st.sweep != Some(sweep) {
                    w.push(base + 1, sweep);
                    self.lanes[lane].sweep = Some(sweep);
                }
                // A note-on writes the high byte: restarts the envelope and
                // resets the duty phase, like a real driver's note-on.
                self.write_period(lane, t, first, w);
            }
            LANE_TRI => {
                let Some(t) = self.period(lane, &f) else { return };
                // Control set + reload 127: the linear counter never runs out;
                // reload 0 silences it (a volume macro step of 0).
                let ctl = if f.volume == 0 { 0x80 } else { 0xFF };
                let unmute = st.ctl == Some(0x80) && ctl == 0xFF;
                if first || st.ctl != Some(ctl) {
                    w.push(0x4008, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
                // The linear counter reloads only after a $400B write.
                if unmute {
                    self.lanes[lane].period = None;
                }
                self.write_period(lane, t, first, w);
            }
            LANE_NOISE => {
                let ctl = self.env_bits(lane, &f, macro_vol);
                if first || st.ctl != Some(ctl) {
                    w.push(0x400C, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
                let per = self.noise_period(lane, &f);
                if first || st.sweep != Some(per) {
                    w.push(0x400E, per);
                    self.lanes[lane].sweep = Some(per);
                }
                if first {
                    w.push(0x400F, LEN);
                }
            }
            LANE_DMC => {
                let s = self.samples[self.dmc_sample(st.play_pitch)];
                // A user sample plays at the rate it was encoded for.
                let rate = s.rate.unwrap_or(self.pv(lane, ParamId::Rate) as u8);
                let ctl = ((self.pv(lane, ParamId::Loop) as u8) << 6) | rate;
                if first {
                    w.push(0x4010, ctl);
                    w.push(0x4011, START_LEVEL);
                    w.push(0x4012, s.addr_reg);
                    w.push(0x4013, s.len_reg);
                    // Clear D first so setting it restarts even mid-sample.
                    w.push(0x4015, 0x0F);
                    w.push(0x4015, 0x1F);
                    self.lanes[lane].ctl = Some(ctl);
                } else if st.ctl != Some(ctl) {
                    w.push(0x4010, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
            }
            LANE_V1 | LANE_V2 => {
                let Some(t) = self.period(lane, &f) else { return };
                let ctl = self.vrc6_ctl(lane, &f, macro_vol);
                if first || st.ctl != Some(ctl) {
                    w.push(if lane == LANE_V1 { 0x9000 } else { 0xA000 }, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
                self.write_period(lane, t, first, w);
            }
            _ => {
                let Some(t) = self.period(lane, &f) else { return };
                let base = self.saw_rate(lane) as u32;
                let rate = if macro_vol { ((base * f.volume as u32 + 7) / 15) as u8 } else { base as u8 };
                if first || st.ctl != Some(rate) {
                    w.push(0xB000, rate);
                    self.lanes[lane].ctl = Some(rate);
                }
                self.write_period(lane, t, first, w);
            }
        }
    }

    fn note_off(&mut self, lane: usize, w: &mut Writes) {
        match lane {
            LANE_P1 | LANE_P2 => {
                let duty = self.lanes[lane].ctl.map_or(0, |c| c & 0xC0);
                w.push(0x4000 + 4 * lane as u16, duty | 0x30);
            }
            // Reloads the linear counter with 0 at the next quarter frame.
            LANE_TRI => w.push(0x4008, 0x80),
            LANE_NOISE => w.push(0x400C, 0x30),
            // One-shot samples finish on their own; a looping one is stopped.
            LANE_DMC => {
                if self.pv(lane, ParamId::Loop) != 0 {
                    w.push(0x4015, 0x0F);
                }
            }
            LANE_V1 | LANE_V2 => {
                let ctl = self.lanes[lane].ctl.unwrap_or(0) & 0xF0;
                w.push(if lane == LANE_V1 { 0x9000 } else { 0xA000 }, ctl);
            }
            _ => w.push(0xB002, 0x00), // clear E: accumulator held at 0
        }
        self.lanes[lane].note = None;
        self.lanes[lane].vol = None;
        self.lanes[lane].ctl = None;
        self.lanes[lane].sweep = None;
        self.lanes[lane].period = None;
        self.lanes[lane].mac = MacroState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn drv() -> Driver {
        Driver::new([SampleRef { present: true, ..SampleRef::default() }; SAMPLE_COUNT])
    }

    fn writes_of(w: &Writes) -> Vec<(u16, u8)> {
        w.as_slice().to_vec()
    }

    /// The duty bits of the last $4000 write.
    fn duty_written(w: &Writes) -> Option<u8> {
        w.as_slice().iter().rev().find(|x| x.0 == 0x4000).map(|x| x.1 >> 6)
    }

    #[test]
    fn clip_instruments_switch_at_note_on_and_never_mid_note() {
        use crate::instrument::Seq;
        // Two instruments that differ only in duty: 12.5% (2/16) and 75% (12/16).
        let thin = Instrument { duty: Seq::new(&[2], NONE, NONE), ..Instrument::PLAIN };
        let wide = Instrument { duty: Seq::new(&[12], NONE, NONE), ..Instrument::PLAIN };
        let bank = [thin, wide];
        let mut d = drv();
        let mut w = Writes::default();
        // Clip 1 (thin) then clip 2 (wide) on Pulse 1; the lane default is plain.
        let notes = [Note { inst: 0, ..Note::new(0, 960, 69) }, Note { inst: 1, ..Note::new(960, 1920, 69) }];
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        lanes[0] = &notes;
        let song = Song { lanes, instruments: &bank, lane_inst: [NONE; LANES], preview_inst: NONE, auto: [&[]; LANES], auto_points: &[] };
        d.tick(&song, None, 0.0, true, &mut w);
        assert_eq!(duty_written(&w), Some(0), "clip 1: 12.5%");
        w.clear();
        d.tick(&song, Some(0.0), 980.0, true, &mut w);
        assert_eq!(duty_written(&w), Some(3), "clip 2's note-on: 75%");
        // A held note keeps its instrument when a new snapshot moves its clip
        // to the other instrument and reorders the bank.
        let mut d = drv();
        w.clear();
        d.tick(&song, None, 0.0, true, &mut w);
        let moved = [Note { inst: 1, ..Note::new(0, 960, 69) }];
        let reordered = [wide, thin];
        let mut lanes2: [&[Note]; LANES] = [&[]; LANES];
        lanes2[0] = &moved;
        let song2 = Song { lanes: lanes2, instruments: &reordered, lane_inst: [NONE; LANES], preview_inst: NONE, auto: [&[]; LANES], auto_points: &[] };
        w.clear();
        for i in 1..10 {
            d.tick(&song2, Some((i - 1) as f64 * 40.0), i as f64 * 40.0, true, &mut w);
        }
        assert_eq!(duty_written(&w), None, "no duty change while the note is held");
        assert_eq!(d.note_ons[0], 1, "and no retrigger");
    }

    #[test]
    fn note_lookup() {
        let notes = [
            Note::new(0, 100, 60),
            Note::new(200, 210, 62),
        ];
        assert_eq!(Driver::note_at(&notes, Some(-1.0), 50.0).map(|n| n.pitch), Some(60));
        assert_eq!(Driver::note_at(&notes, Some(50.0), 150.0), None);
        // A 10-tick note between two frame ticks still plays for one frame.
        assert_eq!(Driver::note_at(&notes, Some(150.0), 230.0).map(|n| n.pitch), Some(62));
        assert_eq!(Driver::note_at(&notes, Some(230.0), 270.0), None);
        // After a seek, an ended note is not replayed; a running one is chased.
        assert_eq!(Driver::note_at(&notes, None, 230.0), None);
        assert_eq!(Driver::note_at(&notes, None, 50.0).map(|n| n.pitch), Some(60));
    }

    #[test]
    fn pulse_note_on_sustain_off() {
        let mut d = drv();
        let notes = [Note::new(0, 960, 69)];
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        lanes[0] = &notes;
        let song = Song::plain(lanes);
        let mut w = Writes::default();
        d.tick(&song, None, 0.0, true, &mut w);
        let all = writes_of(&w);
        // Note-on writes for pulse 1: control, sweep, period low, period high.
        let p1: Vec<_> = all.iter().filter(|(a, _)| (0x4000..0x4004).contains(a)).cloned().collect();
        assert_eq!(p1[p1.len() - 4..], [(0x4000, 0x80 | 0x30 | 12), (0x4001, 0x08), (0x4002, 253), (0x4003, 0x08)]);
        assert_eq!(d.note_ons[0], 1);

        // Sustain: nothing written unless a parameter changes.
        w.clear();
        d.tick(&song, Some(0.0), 40.0, true, &mut w);
        assert!(w.as_slice().is_empty());
        d.params.set(0, ParamId::Volume, 5);
        d.tick(&song, Some(40.0), 80.0, true, &mut w);
        assert_eq!(writes_of(&w), [(0x4000, 0x80 | 0x30 | 5)], "volume change, no phase reset");

        // Note end -> constant volume 0.
        w.clear();
        d.tick(&song, Some(80.0), 970.0, true, &mut w);
        assert_eq!(writes_of(&w), [(0x4000, 0x80 | 0x30)]);
        assert_eq!(d.sounding, 0);
    }

    #[test]
    fn mute_and_solo() {
        let mut d = drv();
        let notes = [Note::new(0, 960, 40)];
        let song = Song::plain([&notes; LANES]);
        let mut w = Writes::default();
        d.mute = 1 << LANE_TRI;
        d.tick(&song, None, 0.0, true, &mut w);
        assert_eq!(d.sounding, !(1u8 << LANE_TRI));
        d.solo = 1 << LANE_SAW;
        d.tick(&song, Some(0.0), 10.0, true, &mut w);
        assert_eq!(d.sounding, 1 << LANE_SAW);
        d.vrc6 = false;
        d.tick(&song, Some(10.0), 20.0, true, &mut w);
        assert_eq!(d.sounding, 0, "VRC6 lanes are silent without the VRC6");
    }

    #[test]
    fn edits_elsewhere_dont_retrigger() {
        let mut d = drv();
        let a = [Note::new(0, 960, 60)];
        let b = [
            Note::new(0, 960, 60),
            Note::new(2000, 2100, 64),
        ];
        let mut w = Writes::default();
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        lanes[0] = &a;
        d.tick(&Song::plain(lanes), None, 10.0, true, &mut w);
        w.clear();
        lanes[0] = &b;
        d.tick(&Song::plain(lanes), Some(10.0), 50.0, true, &mut w);
        assert!(w.as_slice().is_empty());
        assert_eq!(d.note_ons[0], 1);
    }

    #[test]
    fn per_note_volume_is_scaled_by_the_channel_and_updates_while_held() {
        let mut d = drv();
        // Channel volume 15 passes the note's volume through unchanged.
        d.params.set(0, ParamId::Volume, 15);
        let mut notes = [Note { volume: Some(3), ..Note::new(0, 960, 69) }];
        let mut w = Writes::default();
        {
            let mut lanes: [&[Note]; LANES] = [&[]; LANES];
            lanes[0] = &notes;
            d.tick(&Song::plain(lanes), None, 0.0, true, &mut w);
        }
        assert!(w.as_slice().contains(&(0x4000, 0x80 | 0x30 | 3)));
        // Editing the volume of the sounding note: no retrigger, new V.
        notes[0].volume = Some(9);
        w.clear();
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        lanes[0] = &notes;
        d.tick(&Song::plain(lanes), Some(0.0), 40.0, true, &mut w);
        assert_eq!(writes_of(&w), [(0x4000, 0x80 | 0x30 | 9)]);
        assert_eq!(d.note_ons[0], 1);
        // Channel volume 10 scales it: ceil(9 * 10 / 15) = 6, still no retrigger.
        d.params.set(0, ParamId::Volume, 10);
        w.clear();
        d.tick(&Song::plain(lanes), Some(40.0), 80.0, true, &mut w);
        assert_eq!(writes_of(&w), [(0x4000, 0x80 | 0x30 | 6)]);
        // A note without a volume plays at the channel volume (as before).
        let plain = [Note::new(0, 960, 69)];
        let mut d = drv();
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        lanes[0] = &plain;
        w.clear();
        d.tick(&Song::plain(lanes), None, 0.0, true, &mut w);
        assert!(w.as_slice().contains(&(0x4000, 0x80 | 0x30 | 12)));
        assert_eq!(d.note_ons[0], 1);
        assert_eq!(saw_rate_for_volume(15), 42);
        assert_eq!(saw_rate_for_volume(0), 0);
    }

    #[test]
    fn preview_plays_only_while_stopped_and_ignores_mute() {
        let mut d = drv();
        let mut w = Writes::default();
        d.mute = 0xFF;
        d.preview[LANE_TRI] = Some(Note::new(1, u32::MAX, 45));
        d.tick(&Song::EMPTY, None, 0.0, false, &mut w);
        assert_eq!(d.sounding, 1 << LANE_TRI);
        assert_eq!(d.note_ons[LANE_TRI], 1);
        // A new serial on the same pitch retriggers.
        d.preview[LANE_TRI] = Some(Note::new(2, u32::MAX, 45));
        d.tick(&Song::EMPTY, None, 0.0, false, &mut w);
        assert_eq!(d.note_ons[LANE_TRI], 2);
        // Playing: previews are ignored.
        d.tick(&Song::EMPTY, Some(0.0), 10.0, true, &mut w);
        assert_eq!(d.sounding, 0);
        // Out of range: refused like song notes (2A03 pulse can't go below A1).
        d.preview[LANE_P1] = Some(Note::new(3, u32::MAX, 20));
        d.tick(&Song::EMPTY, None, 0.0, false, &mut w);
        assert_eq!(d.sounding & 1, 0);
    }

    #[test]
    fn sweep_encoding() {
        let mut d = drv();
        d.params.set(0, ParamId::Sweep, -3);
        d.params.set(0, ParamId::SweepRate, 5);
        assert_eq!(d.pulse_sweep(0), 0x80 | 0x50 | 0x08 | 3);
        d.params.set(0, ParamId::Sweep, 2);
        assert_eq!(d.pulse_sweep(0), 0x80 | 0x50 | 2);
        d.params.set(0, ParamId::Sweep, 0);
        assert_eq!(d.pulse_sweep(0), 0x08);
    }

    /// Batch 5 (AU-07): a locked lane's automation is evaluated once per
    /// frame tick and quantised to the register: the volume nibble written
    /// each frame equals the rounded envelope at that frame's position, and
    /// a duty envelope plays only the 2A03's four duties. With no envelope
    /// the writes are the plain ones.
    #[test]
    fn locked_automation_writes_the_quantised_envelope_each_frame() {
        use crate::auto::{eval, quantise, AutoEnv, AutoPoint, AutoTarget};
        let notes = [Note::new(0, 7680, 69)];
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        lanes[0] = &notes;
        let points = [
            AutoPoint { tick: 0, value: 0.0, curve: 0.0 },
            AutoPoint { tick: 3840, value: 15.0, curve: 0.4 },
            AutoPoint { tick: 0, value: 0.2, curve: 0.0 },
            AutoPoint { tick: 7680, value: 3.4, curve: 0.0 },
        ];
        let envs = [AutoEnv { target: AutoTarget::Chip(ParamId::Volume), start: 0, len: 2 }, AutoEnv { target: AutoTarget::Chip(ParamId::Duty), start: 2, len: 2 }];
        let mut auto: [&[AutoEnv]; LANES] = [&[]; LANES];
        auto[0] = &envs;
        let song = Song { auto, auto_points: &points, ..Song::plain(lanes) };
        let mut d = drv();
        let mut w = Writes::default();
        let (mut vol, mut duty) = (None, None);
        let mut prev = None;
        let mut frames = 0;
        let mut pos = 0.0;
        while pos < 7600.0 {
            w.clear();
            d.tick(&song, prev, pos, true, &mut w);
            for &(a, v) in w.as_slice() {
                if a == 0x4000 {
                    vol = Some(v & 15);
                    duty = Some(v >> 6);
                }
            }
            let want_v = quantise(eval(&points[..2], pos).unwrap(), 0, 15) as u8;
            let want_d = quantise(eval(&points[2..], pos).unwrap(), 0, 3) as u8;
            // Constant-volume mode: out = note (15) x channel volume.
            assert_eq!(vol, Some(crate::instrument::scale_volume(15, want_v)), "frame {frames} at {pos}");
            assert_eq!(duty, Some(want_d), "frame {frames}");
            assert_eq!(d.effective(0, ParamId::Volume), want_v as i32);
            prev = Some(pos);
            pos += 39.93; // a frame tick at 150 BPM
            frames += 1;
        }
        assert!(frames > 180);
        // The envelope is gone: the channel's own Volume plays again.
        let plain = Song::plain(lanes);
        w.clear();
        d.tick(&plain, prev, pos, true, &mut w);
        assert_eq!(d.effective(0, ParamId::Volume), d.params.get(0, ParamId::Volume));
    }

    #[test]
    fn param_ranges_clamp() {
        let mut p = Params::default();
        p.set(LANE_V1, ParamId::Duty, 99);
        assert_eq!(p.get(LANE_V1, ParamId::Duty), 8);
        p.set(LANE_P1, ParamId::Duty, 99);
        assert_eq!(p.get(LANE_P1, ParamId::Duty), 3);
        p.set(LANE_SAW, ParamId::Rate, 99);
        assert_eq!(p.get(LANE_SAW, ParamId::Rate), 63);
    }
}
