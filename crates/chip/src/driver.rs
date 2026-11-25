//! The sound driver: turns notes into register writes once per video frame,
//! like a game's music routine running in the NMI handler.
//!
//! - Runs on frame ticks (NTSC: every 29780.5 CPU cycles = 60.0988 Hz).
//! - Each tick, for each lane, it looks up the note under the song position
//!   and issues note-on / note-off / parameter writes. So note starts and ends
//!   are quantised to frame ticks (up to 16.6 ms), exactly as on hardware.
//! - Only controls the hardware has: see `ParamId` and `ENGINE.md`.
//! - No allocation: writes go into a fixed-size `Writes` buffer.

use crate::pitch::{self, Osc};
use crate::samples::{SampleRef, SAMPLE_COUNT, START_LEVEL};

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
}

impl Note {
    pub const fn new(start: u32, end: u32, pitch: u8) -> Self {
        Self { start, end, pitch, volume: None }
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
}

impl Song<'_> {
    pub const EMPTY: Song<'static> = Song { lanes: [&[]; LANES] };
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
pub struct Writes {
    buf: [(u16, u8); 64],
    len: usize,
}

impl Default for Writes {
    fn default() -> Self {
        Self {
            buf: [(0, 0); 64],
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
    sweep: Option<u8>,
}

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
        }
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
        if !self.initialised {
            self.initialised = true;
            w.push(0x4015, 0x0F);
            w.push(0x4017, 0x40); // 4-step, IRQ inhibit
            w.push(0x4001, 0x08);
            w.push(0x4005, 0x08);
            w.push(0x4008, 0x80);
            w.push(0x9003, 0x00);
        }
        let mut sounding = 0u8;
        for lane in 0..LANES {
            let vrc6_lane = lane >= LANE_V1;
            let chip_present = !vrc6_lane || self.vrc6;
            let want = if playing && self.audible(lane) && chip_present {
                Self::note_at(song.lanes[lane], prev, pos)
                    .filter(|n| self.playable(lane, n.pitch))
            } else if !playing && chip_present {
                self.preview[lane].filter(|n| self.playable(lane, n.pitch))
            } else {
                None
            };
            if let Some(n) = want {
                self.lanes[lane].vol = n.volume;
            }
            let cur = self.lanes[lane].note;
            match want {
                Some(n) if cur != Some((n.start, n.pitch)) => {
                    self.note_on(lane, n.pitch, w);
                    self.lanes[lane].note = Some((n.start, n.pitch));
                    self.note_ons[lane] = self.note_ons[lane].wrapping_add(1);
                }
                Some(_) => self.sustain(lane, w),
                None if cur.is_some() => self.note_off(lane, w),
                None => {}
            }
            if self.lanes[lane].note.is_some() {
                sounding |= 1 << lane;
            }
        }
        self.sounding = sounding;
    }

    fn playable(&self, lane: usize, pitch: u8) -> bool {
        match lane {
            LANE_P1 | LANE_P2 => pitch::period_for(Osc::Pulse, pitch).is_some(),
            LANE_TRI => pitch::period_for(Osc::Triangle, pitch).is_some(),
            LANE_V1 | LANE_V2 => pitch::period_for(Osc::Vrc6Pulse, pitch).is_some(),
            LANE_SAW => pitch::period_for(Osc::Saw, pitch).is_some(),
            _ => true,
        }
    }

    /// V for `lane`: the sounding note's volume, else the lane's parameter.
    fn volume(&self, lane: usize) -> u8 {
        self.lanes[lane]
            .vol
            .map_or(self.params.get(lane, ParamId::Volume) as u8, |v| v.min(15))
    }

    /// The saw's accumulator rate: from the note's volume if it has one.
    fn saw_rate(&self, lane: usize) -> u8 {
        self.lanes[lane]
            .vol
            .map_or(self.params.get(lane, ParamId::Rate) as u8, saw_rate_for_volume)
    }

    fn env_bits(&self, lane: usize) -> u8 {
        let p = &self.params;
        let v = self.volume(lane);
        match p.get(lane, ParamId::Envelope) {
            0 => 0x30 | v, // halt length, constant volume
            1 => v,        // length runs (254 half frames), decay
            _ => 0x20 | v, // halt length + loop, decay
        }
    }

    fn pulse_ctl(&self, lane: usize) -> u8 {
        ((self.params.get(lane, ParamId::Duty) as u8) << 6) | self.env_bits(lane)
    }

    fn pulse_sweep(&self, lane: usize) -> u8 {
        let s = self.params.get(lane, ParamId::Sweep);
        if s == 0 {
            // Negate set: the target can never overflow, so low notes aren't muted.
            0x08
        } else {
            let p = self.params.get(lane, ParamId::SweepRate) as u8;
            0x80 | (p << 4) | if s < 0 { 0x08 } else { 0 } | s.unsigned_abs() as u8
        }
    }

    fn vrc6_ctl(&self, lane: usize) -> u8 {
        let d = self.params.get(lane, ParamId::Duty);
        let v = self.volume(lane);
        if d >= 8 {
            0x80 | 0x70 | v
        } else {
            ((d as u8) << 4) | v
        }
    }

    fn noise_period(&self, pitch: u8) -> u8 {
        let p = self.params.get(LANE_NOISE, ParamId::NoisePeriod);
        let mode = (self.params.get(LANE_NOISE, ParamId::NoiseMode) as u8) << 7;
        mode | if p < 0 { pitch::noise_index_for(pitch) } else { p as u8 }
    }

    fn note_on(&mut self, lane: usize, pitch: u8, w: &mut Writes) {
        // Length index 1 = 254 half frames (2.1 s): only matters in decay mode,
        // where the envelope reaches 0 within 1 s anyway.
        const LEN: u8 = 1 << 3;
        match lane {
            LANE_P1 | LANE_P2 => {
                let base = 0x4000 + 4 * lane as u16;
                let t = pitch::period_for(Osc::Pulse, pitch).unwrap_or(0);
                let ctl = self.pulse_ctl(lane);
                let sweep = self.pulse_sweep(lane);
                w.push(base, ctl);
                w.push(base + 1, sweep);
                w.push(base + 2, t as u8);
                // Restarts the envelope and resets the duty phase, like a real driver's note-on.
                w.push(base + 3, LEN | (t >> 8) as u8);
                self.lanes[lane].ctl = Some(ctl);
                self.lanes[lane].sweep = Some(sweep);
            }
            LANE_TRI => {
                let t = pitch::period_for(Osc::Triangle, pitch).unwrap_or(0);
                // Control set + reload 127: the linear counter never runs out.
                w.push(0x4008, 0xFF);
                w.push(0x400A, t as u8);
                w.push(0x400B, LEN | (t >> 8) as u8);
            }
            LANE_NOISE => {
                let ctl = self.env_bits(lane);
                w.push(0x400C, ctl);
                w.push(0x400E, self.noise_period(pitch));
                w.push(0x400F, LEN);
                self.lanes[lane].ctl = Some(ctl);
            }
            LANE_DMC => {
                let sel = self.params.get(lane, ParamId::Sample);
                let idx = if sel < 0 { pitch as usize % SAMPLE_COUNT } else { sel as usize };
                let s = self.samples[idx];
                let rate = self.params.get(lane, ParamId::Rate) as u8;
                let lp = (self.params.get(lane, ParamId::Loop) as u8) << 6;
                w.push(0x4010, lp | rate);
                w.push(0x4011, START_LEVEL);
                w.push(0x4012, s.addr_reg);
                w.push(0x4013, s.len_reg);
                // Clear D first so setting it restarts even mid-sample.
                w.push(0x4015, 0x0F);
                w.push(0x4015, 0x1F);
                self.lanes[lane].ctl = Some(lp | rate);
            }
            LANE_V1 | LANE_V2 => {
                let base = if lane == LANE_V1 { 0x9000 } else { 0xA000 };
                let t = pitch::period_for(Osc::Vrc6Pulse, pitch).unwrap_or(0);
                let ctl = self.vrc6_ctl(lane);
                w.push(base, ctl);
                w.push(base + 1, t as u8);
                w.push(base + 2, 0x80 | (t >> 8) as u8);
                self.lanes[lane].ctl = Some(ctl);
            }
            _ => {
                let t = pitch::period_for(Osc::Saw, pitch).unwrap_or(0);
                let rate = self.saw_rate(lane);
                w.push(0xB000, rate);
                w.push(0xB001, t as u8);
                w.push(0xB002, 0x80 | (t >> 8) as u8);
                self.lanes[lane].ctl = Some(rate);
            }
        }
    }

    /// While a note holds: apply parameter changes (no phase reset).
    fn sustain(&mut self, lane: usize, w: &mut Writes) {
        let st = self.lanes[lane];
        match lane {
            LANE_P1 | LANE_P2 => {
                let base = 0x4000 + 4 * lane as u16;
                let ctl = self.pulse_ctl(lane);
                if st.ctl != Some(ctl) {
                    w.push(base, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
                let sweep = self.pulse_sweep(lane);
                if st.sweep != Some(sweep) {
                    w.push(base + 1, sweep);
                    self.lanes[lane].sweep = Some(sweep);
                }
            }
            LANE_NOISE => {
                let ctl = self.env_bits(lane);
                if st.ctl != Some(ctl) {
                    w.push(0x400C, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
            }
            LANE_V1 | LANE_V2 => {
                let ctl = self.vrc6_ctl(lane);
                if st.ctl != Some(ctl) {
                    w.push(if lane == LANE_V1 { 0x9000 } else { 0xA000 }, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
            }
            LANE_SAW => {
                let rate = self.saw_rate(lane);
                if st.ctl != Some(rate) {
                    w.push(0xB000, rate);
                    self.lanes[lane].ctl = Some(rate);
                }
            }
            LANE_DMC => {
                let ctl = ((self.params.get(lane, ParamId::Loop) as u8) << 6)
                    | self.params.get(lane, ParamId::Rate) as u8;
                if st.ctl != Some(ctl) {
                    w.push(0x4010, ctl);
                    self.lanes[lane].ctl = Some(ctl);
                }
            }
            _ => {}
        }
    }

    fn note_off(&mut self, lane: usize, w: &mut Writes) {
        match lane {
            LANE_P1 | LANE_P2 => {
                let duty = (self.params.get(lane, ParamId::Duty) as u8) << 6;
                w.push(0x4000 + 4 * lane as u16, duty | 0x30);
            }
            // Reloads the linear counter with 0 at the next quarter frame.
            LANE_TRI => w.push(0x4008, 0x80),
            LANE_NOISE => w.push(0x400C, 0x30),
            // One-shot samples finish on their own; a looping one is stopped.
            LANE_DMC => {
                if self.params.get(lane, ParamId::Loop) != 0 {
                    w.push(0x4015, 0x0F);
                }
            }
            LANE_V1 | LANE_V2 => {
                let ctl = self.vrc6_ctl(lane) & 0xF0;
                w.push(if lane == LANE_V1 { 0x9000 } else { 0xA000 }, ctl);
            }
            _ => w.push(0xB002, 0x00), // clear E: accumulator held at 0
        }
        self.lanes[lane].note = None;
        self.lanes[lane].vol = None;
        self.lanes[lane].ctl = None;
        self.lanes[lane].sweep = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn drv() -> Driver {
        Driver::new([SampleRef::default(); SAMPLE_COUNT])
    }

    fn writes_of(w: &Writes) -> Vec<(u16, u8)> {
        w.as_slice().to_vec()
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
        let song = Song { lanes };
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
        let song = Song { lanes: [&notes; LANES] };
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
        d.tick(&Song { lanes }, None, 10.0, true, &mut w);
        w.clear();
        lanes[0] = &b;
        d.tick(&Song { lanes }, Some(10.0), 50.0, true, &mut w);
        assert!(w.as_slice().is_empty());
        assert_eq!(d.note_ons[0], 1);
    }

    #[test]
    fn per_note_volume_overrides_the_lane_and_updates_while_held() {
        let mut d = drv();
        let mut notes = [Note { volume: Some(3), ..Note::new(0, 960, 69) }];
        let mut w = Writes::default();
        {
            let mut lanes: [&[Note]; LANES] = [&[]; LANES];
            lanes[0] = &notes;
            d.tick(&Song { lanes }, None, 0.0, true, &mut w);
        }
        assert!(w.as_slice().contains(&(0x4000, 0x80 | 0x30 | 3)));
        // Editing the volume of the sounding note: no retrigger, new V.
        notes[0].volume = Some(9);
        w.clear();
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        lanes[0] = &notes;
        d.tick(&Song { lanes }, Some(0.0), 40.0, true, &mut w);
        assert_eq!(writes_of(&w), [(0x4000, 0x80 | 0x30 | 9)]);
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
