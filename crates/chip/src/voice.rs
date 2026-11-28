//! One studio voice: a single chip channel of one voice type, driven by the
//! sound driver's own per-lane code (BATCH4A_PROMPT.md §4, VISION.md "Tracks
//! are instruments").
//!
//! **No second emulator.** A voice owns the same clean-room channel units the
//! console uses (`Pulse`, `Triangle`, `Noise`, `Dmc`, the VRC6 pulse and saw)
//! and a `Driver` of which it uses one lane (`Driver::tick_lane`), so a note
//! on a studio track goes through exactly the code a locked track's note does:
//! the same register writes, macros, envelopes, length counters and timers.
//! What differs is around it: each voice has its own frame counter, and its
//! output is mixed **linearly** into a shared step buffer, each voice through
//! its own channel's DAC curve *alone* (`level_of`): a pulse voice at volume v
//! contributes `pulse_out(v, 0)`, exactly what that note on Pulse 1 gives with
//! nothing else playing. The console's non-linear interaction between channels
//! (two pulses together are quieter than the sum) is the hardware's, and only
//! locked tracks have it (BATCH4A.md, "studio vs locked").
//!
//! Contributions are taken relative to the voice's power-up level, so adding an
//! idle voice to a pool (the pool grows from the UI thread) never steps the
//! output.
//!
//! **Bipolar (Batch 5 A3).** A chip channel is unipolar: a pulse swings
//! between 0 and its volume, so every note carries a DC offset of its duty x
//! its level, and the baseline jumped with each note (the console's output
//! filters remove it; the studio bus had only a 5 Hz blocker, too slow to
//! keep a 50 ms mean flat). A voice now also subtracts its waveform's own mean
//! (`Voice::dc`): the duty's share of the volume for the pulses, half of it
//! for the noise, the 32-step average for a running triangle (a halted one
//! holds a level: all DC), the saw's 7-step average, a sample's start level
//! while it plays. The mean changes only where the volume, duty, rate or
//! gating does, so it is one more band-limited step at those moments:
//! deterministic, allocation-free, a few multiplies per change. A silent or
//! idle voice's mean is 0, so a new idle voice still never steps the output.
//!
//! No allocation: everything is fixed-size.

use crate::blep::{Blep, AMP_BITS};
use crate::dmc::{Dmc, ROM_SIZE};
use crate::driver::{Driver, Note, Song, Writes, LANE_DMC, LANE_NOISE, LANE_P1, LANE_SAW, LANE_TRI, LANE_V1};
use crate::frame_counter::FrameCounter;
use crate::mixer;
use crate::noise::Noise;
use crate::pulse::Pulse;
use crate::samples::SampleRef;
use crate::triangle::Triangle;
use crate::vrc6::Vrc6;

/// What a track plays: one of the chip's channel types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VoiceKind {
    Pulse,
    Triangle,
    Noise,
    Dpcm,
    Vrc6Pulse,
    Saw,
}

impl VoiceKind {
    pub const ALL: [VoiceKind; 6] = [VoiceKind::Pulse, VoiceKind::Triangle, VoiceKind::Noise, VoiceKind::Dpcm, VoiceKind::Vrc6Pulse, VoiceKind::Saw];

    /// The driver lane whose code (and register addresses) plays this type:
    /// the first console channel of the type.
    pub const fn lane(self) -> usize {
        match self {
            VoiceKind::Pulse => LANE_P1,
            VoiceKind::Triangle => LANE_TRI,
            VoiceKind::Noise => LANE_NOISE,
            VoiceKind::Dpcm => LANE_DMC,
            VoiceKind::Vrc6Pulse => LANE_V1,
            VoiceKind::Saw => LANE_SAW,
        }
    }

    /// The type of console channel `lane` (0..8).
    pub const fn of_lane(lane: usize) -> VoiceKind {
        match lane {
            0 | 1 => VoiceKind::Pulse,
            2 => VoiceKind::Triangle,
            3 => VoiceKind::Noise,
            4 => VoiceKind::Dpcm,
            5 | 6 => VoiceKind::Vrc6Pulse,
            _ => VoiceKind::Saw,
        }
    }

    /// The largest DAC value (for meters).
    pub const fn full_scale(self) -> u8 {
        match self {
            VoiceKind::Dpcm => 127,
            VoiceKind::Saw => 31,
            _ => 15,
        }
    }
}

/// The voice's output level for DAC value `v`: its channel's curve alone.
pub fn level_of(kind: VoiceKind, v: u8) -> f64 {
    match kind {
        VoiceKind::Pulse => mixer::pulse_out(v, 0),
        VoiceKind::Triangle => mixer::tnd_out(v, 0, 0),
        VoiceKind::Noise => mixer::tnd_out(0, v, 0),
        VoiceKind::Dpcm => mixer::tnd_out(0, 0, v),
        VoiceKind::Vrc6Pulse | VoiceKind::Saw => mixer::VRC6_SIGN * v as f64 * mixer::vrc6_step(),
    }
}

fn fixed(amp: f64) -> i32 {
    (amp * (1u64 << AMP_BITS) as f64) as i32
}

/// Where a voice's output steps go (Batch 4b §4, stereo). A voice's
/// contribution is split three ways: **centre** (heard in both channels),
/// **left only** and **right only**. A centred voice puts everything in the
/// centre, so with one step buffer (`Blep`) nothing changes from the mono
/// build; a panned voice puts the quieter side's share in the centre and the
/// difference in its louder side (two steps per change, not three).
pub trait StepOut {
    /// Band-limited steps of `d` (centre, left only, right only) at `cycle`.
    fn step(&mut self, cycle: u64, d: [i32; 3]);
}

/// One mono step buffer: the centre only (a voice that is never panned).
impl StepOut for Blep {
    fn step(&mut self, cycle: u64, d: [i32; 3]) {
        debug_assert!(d[1] == 0 && d[2] == 0, "a panned voice needs a stereo output");
        self.add(cycle, d[0]);
    }
}

/// The share of a voice's output in (centre, left only, right only) for a
/// track's channel gains `(l, r)` (the pan law): the quieter side's gain goes
/// to the centre, the rest to the louder side. Centred (`l == r == 1`): all
/// centre, exactly `[1, 0, 0]`.
pub fn pan_split(l: f64, r: f64) -> [f64; 3] {
    let c = l.min(r);
    [c, l - c, r - c]
}

#[derive(Clone)]
pub struct Voice {
    pub kind: VoiceKind,
    /// The driver; only lane `kind.lane()` is used.
    pub driver: Driver,
    pulse: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,
    vrc6: Vrc6,
    frame: FrameCounter,
    /// CPU cycles emulated so far (the console's clock).
    pub cycle: u64,
    /// The DAC value now, and at power-up (the zero of the contribution).
    dac: u8,
    dac0: u8,
    /// The waveform's own mean now, in the mix's units relative to the
    /// power-up level (subtracted: the voice is bipolar, Batch 5 A3).
    mean: f64,
    /// The running triangle's 32-step average (relative to the power-up
    /// level), computed once.
    tri_mean: f64,
    /// The contribution fed to the step buffers (fixed point, gain and pan
    /// applied): centre, left only, right only.
    amp: [i32; 3],
    gain: f64,
    /// The pan split (`pan_split`): `[1, 0, 0]` centred.
    pan: [f64; 3],
    /// Peak DAC value since the last `take_peak`.
    peak: u8,
    /// The note this voice was given (start, pitch), kept while it sounds or
    /// releases; the allocator's identity for "the same note".
    pub held: Option<(u32, u8)>,
    /// Note-ons so far (wrapping): the strip light.
    pub note_ons: u32,
    /// The frame tick the voice last started a note (for stealing the oldest
    /// among equally quiet voices).
    pub started_tick: u64,
    writes: Writes,
}

impl Voice {
    /// A silent voice at CPU cycle `cycle` (the console's current cycle).
    pub fn new(kind: VoiceKind, samples: [SampleRef; crate::samples::SAMPLE_COUNT], cycle: u64) -> Self {
        let mut v = Self {
            kind,
            driver: Driver::new(samples),
            pulse: Pulse::new(true),
            triangle: Triangle::default(),
            noise: Noise::default(),
            dmc: Dmc::default(),
            vrc6: Vrc6::default(),
            frame: FrameCounter::default(),
            cycle,
            dac: 0,
            dac0: 0,
            mean: 0.0,
            tri_mean: 0.0,
            amp: [0; 3],
            gain: 1.0,
            pan: [1.0, 0.0, 0.0],
            peak: 0,
            held: None,
            note_ons: 0,
            started_tick: 0,
            writes: Writes::default(),
        };
        v.dac = v.output();
        v.dac0 = v.dac;
        if kind == VoiceKind::Triangle {
            let base = level_of(kind, v.dac0);
            let seq = crate::tables::TRIANGLE_SEQ;
            v.tri_mean = seq.iter().map(|&t| level_of(kind, t) - base).sum::<f64>() / seq.len() as f64;
        }
        // The peak starts at the power-up level (a triangle's is 15, not 0):
        // an idle voice has nothing to report.
        v.peak = v.dac;
        v
    }

    fn output(&self) -> u8 {
        match self.kind {
            VoiceKind::Pulse => self.pulse.output(),
            VoiceKind::Triangle => self.triangle.output(),
            VoiceKind::Noise => self.noise.output(),
            VoiceKind::Dpcm => self.dmc.output(),
            VoiceKind::Vrc6Pulse => self.vrc6.pulse1.output(),
            VoiceKind::Saw => self.vrc6.saw.output(),
        }
    }

    /// The waveform's mean level now (relative to the power-up level): what
    /// the channel's output averages to over one period of its waveform, given
    /// its volume, duty, rate and gating (0 when it is silent). Subtracting it
    /// makes the voice bipolar (Batch 5 A3).
    fn dc(&self) -> f64 {
        let k = self.kind;
        let base = level_of(k, self.dac0);
        match k {
            VoiceKind::Pulse => {
                let p = &self.pulse;
                if !p.len.active() || p.sweep_muted() {
                    return 0.0;
                }
                let ones = crate::tables::PULSE_DUTY[p.duty as usize & 3].iter().filter(|&&b| b != 0).count();
                (level_of(k, p.env.output()) - base) * ones as f64 / 8.0
            }
            VoiceKind::Noise => {
                if !self.noise.len.active() {
                    return 0.0;
                }
                // The 15-bit sequence's output is high on 16383 of its 32767
                // states (the short sequences: about half too).
                (level_of(k, self.noise.env.output()) - base) * 0.5
            }
            VoiceKind::Triangle => {
                let t = &self.triangle;
                if t.linear > 0 && t.len.active() {
                    self.tri_mean
                } else {
                    // Halted: it holds its step, a level with no waveform.
                    level_of(k, t.output()) - base
                }
            }
            VoiceKind::Dpcm => {
                // While a sample plays, the level it started from; after it,
                // the level it stopped at (held: no waveform).
                if self.dmc.sounding() {
                    level_of(k, crate::samples::START_LEVEL) - base
                } else {
                    level_of(k, self.dmc.output()) - base
                }
            }
            VoiceKind::Vrc6Pulse => {
                let p = &self.vrc6.pulse1;
                if !p.enabled {
                    return 0.0;
                }
                let share = if p.mode { 1.0 } else { (p.duty as f64 + 1.0) / 16.0 };
                (level_of(k, p.volume) - base) * share
            }
            VoiceKind::Saw => {
                let s = &self.vrc6.saw;
                if !s.enabled {
                    return 0.0;
                }
                // Seven accumulator states (0, r, 2r ... 6r, wrapping), each
                // held for two clocks.
                let sum: u32 = (0..7u32).map(|j| ((j * s.rate as u32) & 0xFF) >> 3).sum();
                (level_of(k, 1) - level_of(k, 0)) * sum as f64 / 7.0
            }
        }
    }

    /// The fixed-point contribution for the current DAC value, gain and pan.
    /// Centred, the centre share is `fixed(x * 1.0)`: exactly the mono
    /// build's `fixed(x)`.
    fn amp_now(&self) -> [i32; 3] {
        let x = (level_of(self.kind, self.dac) - level_of(self.kind, self.dac0) - self.mean) * self.gain;
        [fixed(x * self.pan[0]), fixed(x * self.pan[1]), fixed(x * self.pan[2])]
    }

    fn emit<S: StepOut>(&mut self, out: &mut S) {
        let a = self.amp_now();
        out.step(self.cycle, [a[0] - self.amp[0], a[1] - self.amp[1], a[2] - self.amp[2]]);
        self.amp = a;
    }

    /// After a register write or a frame-counter clock: the output and the
    /// waveform's mean (its inputs, the volume, duty, rate and gating, change
    /// only there).
    fn refresh<S: StepOut>(&mut self, out: &mut S) {
        let o = self.output();
        let m = self.dc();
        if o == self.dac && m == self.mean {
            return;
        }
        self.dac = o;
        self.mean = m;
        self.peak = self.peak.max(o);
        self.emit(out);
    }

    /// After a timer clock: only the output moves along the waveform (the
    /// mean stays), except a sample's end, which a DMC clock decides.
    fn refresh_wave<S: StepOut>(&mut self, out: &mut S) {
        if self.kind == VoiceKind::Dpcm {
            return self.refresh(out);
        }
        let o = self.output();
        if o == self.dac {
            return;
        }
        self.dac = o;
        self.peak = self.peak.max(o);
        self.emit(out);
    }

    /// Whether the voice contributes nothing: its level minus its mean is 0
    /// (idle at power-up, or a halted triangle holding a step).
    fn silent(&self) -> bool {
        level_of(self.kind, self.dac) - level_of(self.kind, self.dac0) - self.mean == 0.0
    }

    /// Changes the voice's gain at the current cycle (a band-limited step of
    /// the difference).
    pub fn set_gain<S: StepOut>(&mut self, g: f64, out: &mut S) {
        if g == self.gain {
            return;
        }
        self.gain = g;
        self.emit(out);
    }

    /// Changes the voice's pan split (`pan_split`) at the current cycle: its
    /// contribution moves between the centre and the sides as one step.
    pub fn set_pan<S: StepOut>(&mut self, p: [f64; 3], out: &mut S) {
        if p == self.pan {
            return;
        }
        self.pan = p;
        self.emit(out);
    }

    pub fn pan(&self) -> [f64; 3] {
        self.pan
    }

    /// The voice's pitch offset in semitones (unison detune and drift; 0.0 =
    /// the note's own pitch). Heard from the next frame tick.
    pub fn set_detune(&mut self, semitones: f64) {
        self.driver.detune[self.kind.lane()] = semitones;
    }

    /// The voice's output (gain applied, no pan) at its peak DAC value since
    /// the last `take_peak`, in the mix's units (full scale 1.0): the track's
    /// clip mark adds these up (an upper bound of the track's own peak).
    pub fn peak_out(&self) -> f64 {
        if self.free() {
            return 0.0;
        }
        ((level_of(self.kind, self.peak) - level_of(self.kind, self.dac0)) * self.gain).abs()
    }

    pub fn gain(&self) -> f64 {
        self.gain
    }

    /// Sets the gain of a silent voice (at its power-up level nothing changes).
    pub fn set_gain_silently(&mut self, g: f64) -> bool {
        let silent = self.silent();
        if silent {
            self.gain = g;
        }
        silent
    }

    /// Sets the pan split of a silent voice (nothing to move).
    pub fn set_pan_silently(&mut self, p: [f64; 3]) -> bool {
        let silent = self.silent();
        if silent {
            self.pan = p;
        }
        silent
    }

    /// Moves a voice that has never run onto the console's clock (a fresh
    /// voice in a new pool starts at the current cycle).
    pub fn resync(&mut self, cycle: u64) {
        if self.cycle < cycle && self.free() && self.silent() {
            self.cycle = cycle;
        }
    }

    /// The voice's register write at the current cycle: the console's address
    /// map, for this voice's channel only.
    fn write(&mut self, addr: u16, v: u8, rom: &[u8; ROM_SIZE]) {
        match (self.kind, addr) {
            (VoiceKind::Pulse, 0x4000..=0x4003) => self.pulse.write(addr & 3, v),
            (VoiceKind::Triangle, 0x4008..=0x400B) => self.triangle.write(addr & 3, v),
            (VoiceKind::Noise, 0x400C..=0x400F) => self.noise.write(addr & 3, v),
            (VoiceKind::Dpcm, 0x4010..=0x4013) => self.dmc.write(addr & 3, v),
            (_, 0x4015) => match self.kind {
                VoiceKind::Pulse => self.pulse.len.set_enabled(v & 1 != 0),
                VoiceKind::Triangle => self.triangle.len.set_enabled(v & 4 != 0),
                VoiceKind::Noise => self.noise.len.set_enabled(v & 8 != 0),
                VoiceKind::Dpcm => self.dmc.set_enabled(v & 0x10 != 0, rom),
                _ => {}
            },
            (_, 0x4017) => self.frame.write(v, self.cycle),
            (VoiceKind::Vrc6Pulse, 0x9000..=0x9003) | (VoiceKind::Saw, 0x9003 | 0xB000..=0xB002) => self.vrc6.write(addr, v),
            _ => {}
        }
    }

    fn timer_left(&self) -> u32 {
        match self.kind {
            VoiceKind::Pulse => self.pulse.timer_left,
            VoiceKind::Triangle => self.triangle.timer_left,
            VoiceKind::Noise => self.noise.timer_left,
            VoiceKind::Dpcm => self.dmc.timer_left,
            VoiceKind::Vrc6Pulse => self.vrc6.pulse1.timer_left,
            VoiceKind::Saw => self.vrc6.saw.timer_left,
        }
    }

    /// Whether the channel's output can't change until the next write or
    /// frame clock (its timer then needn't run; the phase catches up).
    fn can_sleep(&self) -> bool {
        match self.kind {
            VoiceKind::Pulse => self.pulse.can_sleep(),
            VoiceKind::Triangle => self.triangle.can_sleep(),
            VoiceKind::Noise => self.noise.can_sleep(),
            VoiceKind::Dpcm => !self.dmc.active() && self.dmc.bytes_remaining == 0,
            VoiceKind::Vrc6Pulse => self.vrc6.halt || self.vrc6.pulse1.can_sleep(),
            VoiceKind::Saw => self.vrc6.halt || self.vrc6.saw.can_sleep(),
        }
    }

    fn catch_up(&mut self, e: u64) {
        let sh = self.vrc6.shift;
        match self.kind {
            VoiceKind::Pulse => self.pulse.catch_up(e),
            VoiceKind::Triangle => self.triangle.catch_up(e),
            VoiceKind::Noise => self.noise.catch_up(e),
            VoiceKind::Dpcm => {}
            VoiceKind::Vrc6Pulse if !self.vrc6.halt => self.vrc6.pulse1.catch_up(e, sh),
            VoiceKind::Saw if !self.vrc6.halt => self.vrc6.saw.catch_up(e, sh),
            _ => {}
        }
    }

    /// Emulates up to (excluding) CPU cycle `target`, feeding output changes
    /// into `blep`.
    pub fn run<S: StepOut>(&mut self, target: u64, rom: &[u8; ROM_SIZE], blep: &mut S) {
        while self.cycle < target {
            let asleep = self.can_sleep();
            let mut dt = (target - self.cycle).min(u32::MAX as u64) as u32;
            dt = dt.min(self.frame.left());
            if !asleep {
                dt = dt.min(self.timer_left());
            }
            if asleep {
                self.catch_up(dt as u64);
            }
            self.cycle += dt as u64;
            let mut dirty = false;
            let mut frame_dirty = false;
            if !asleep {
                let sh = self.vrc6.shift;
                match self.kind {
                    VoiceKind::Pulse => {
                        self.pulse.timer_left -= dt;
                        if self.pulse.timer_left == 0 {
                            self.pulse.clock_timer();
                            dirty = true;
                        }
                    }
                    VoiceKind::Triangle => {
                        self.triangle.timer_left -= dt;
                        if self.triangle.timer_left == 0 {
                            dirty |= self.triangle.clock_timer();
                        }
                    }
                    VoiceKind::Noise => {
                        self.noise.timer_left -= dt;
                        if self.noise.timer_left == 0 {
                            dirty |= self.noise.clock_timer();
                        }
                    }
                    VoiceKind::Dpcm => {
                        self.dmc.timer_left -= dt;
                        if self.dmc.timer_left == 0 {
                            dirty |= self.dmc.clock_timer(rom);
                        }
                    }
                    VoiceKind::Vrc6Pulse => {
                        self.vrc6.pulse1.timer_left -= dt;
                        if self.vrc6.pulse1.timer_left == 0 {
                            dirty |= self.vrc6.pulse1.clock_timer(sh);
                        }
                    }
                    VoiceKind::Saw => {
                        self.vrc6.saw.timer_left -= dt;
                        if self.vrc6.saw.timer_left == 0 {
                            dirty |= self.vrc6.saw.clock_timer(sh);
                        }
                    }
                }
            }
            let c = self.frame.advance(dt);
            if c.quarter || c.half {
                match self.kind {
                    VoiceKind::Pulse => {
                        if c.quarter {
                            self.pulse.env.clock();
                        }
                        if c.half {
                            self.pulse.clock_half();
                        }
                    }
                    VoiceKind::Triangle => {
                        if c.quarter {
                            self.triangle.clock_quarter();
                        }
                        if c.half {
                            self.triangle.len.clock();
                        }
                    }
                    VoiceKind::Noise => {
                        if c.quarter {
                            self.noise.env.clock();
                        }
                        if c.half {
                            self.noise.len.clock();
                        }
                    }
                    _ => {}
                }
                frame_dirty = true;
            }
            if frame_dirty {
                self.refresh(blep);
            } else if dirty {
                self.refresh_wave(blep);
            }
        }
    }

    /// One frame tick at the current cycle: the driver's lane code for `want`
    /// (the note the allocator gave this voice, `None`: release), then its
    /// writes. `params` is the track's channel controls (the driver's lane row).
    #[allow(clippy::too_many_arguments)]
    pub fn tick<S: StepOut>(&mut self, song: &Song, want: Option<Note>, base: u8, audible: bool, params: &[i8; crate::driver::ParamId::ALL.len()], tick: u64, rom: &[u8; ROM_SIZE], blep: &mut S) {
        let lane = self.kind.lane();
        self.driver.params.values[lane] = *params;
        let before = self.driver.note_ons[lane];
        self.writes.clear();
        // The driver's power-up writes ($4015, $4017, sweep negate, triangle
        // control) come with its first tick of any lane.
        self.driver.tick_init(&mut self.writes);
        self.driver.tick_lane(lane, song, want, base, audible, &mut self.writes);
        let w = core::mem::take(&mut self.writes);
        for &(a, v) in w.as_slice() {
            self.write(a, v, rom);
        }
        self.writes = w;
        self.refresh(blep);
        if self.driver.note_ons[lane] != before {
            self.note_ons = self.note_ons.wrapping_add(1);
            self.started_tick = tick;
        }
        self.held = match (self.driver.lane_note(lane), want) {
            (None, _) => None,
            (Some(_), Some(n)) => Some((n.start, n.pitch)),
            (Some(_), None) => self.held,
        };
    }

    /// Whether the voice is silent and free (no note, no release tail).
    pub fn free(&self) -> bool {
        self.driver.lane_note(self.kind.lane()).is_none()
    }

    /// Whether the voice is playing out a release tail (its note ended).
    pub fn releasing(&self) -> bool {
        self.driver.lane_releasing(self.kind.lane())
    }

    /// The voice's loudness now, for stealing the quietest: its DAC level
    /// (0 when free).
    pub fn loudness(&self) -> u8 {
        if self.free() {
            0
        } else {
            self.dac.abs_diff(self.dac0).max(self.peak.abs_diff(self.dac0))
        }
    }

    /// Peak DAC level since the last call, 0..1.
    pub fn take_peak(&mut self) -> f32 {
        // A free voice makes no sound (a halted triangle holds a step: DC,
        // not a level).
        if self.free() {
            self.peak = self.dac;
            return 0.0;
        }
        let full = self.kind.full_scale() as f32;
        let v = if self.kind == VoiceKind::Dpcm { self.peak.abs_diff(self.dac0) as f32 / 64.0 } else { self.peak as f32 / full };
        self.peak = self.dac;
        v.min(1.0)
    }

    /// Installs imported DPCM samples (their bytes are in the console's ROM).
    pub fn set_user_samples(&mut self, refs: [SampleRef; crate::samples::USER_SLOTS]) {
        self.driver.set_user_samples(refs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blep::Blep;
    use crate::driver::{Driver, Song, LANES};
    use crate::synth::Synth;
    use crate::tables::{CPU_CYCLES_PER_FRAME_X2, CPU_HZ_NTSC};
    use std::boxed::Box;
    use std::vec;

    /// A studio voice playing a note writes exactly the registers the console's
    /// lane writes for the same note (one implementation of the driver).
    #[test]
    fn a_voice_runs_the_consoles_lane_code() {
        let samples = crate::samples::build(&mut [0u8; ROM_SIZE]);
        for kind in VoiceKind::ALL {
            let lane = kind.lane();
            let notes = [Note::new(0, 9600, if kind == VoiceKind::Dpcm { 36 } else { 57 })];
            let mut lanes: [&[Note]; LANES] = [&[]; LANES];
            lanes[lane] = &notes;
            let song = Song::plain(lanes);
            let mut console = Driver::new(samples);
            let mut cw = Writes::default();
            console.tick(&song, None, 0.0, true, &mut cw);
            let mut v = Voice::new(kind, samples, 0);
            let mut blep = Box::new(Blep::new(CPU_HZ_NTSC, 48_000.0));
            let rom = Box::new([0u8; ROM_SIZE]);
            v.tick(&Song::EMPTY, Some(notes[0]), crate::instrument::NONE, true, &console.params.values[lane], 0, &rom, &mut *blep);
            // The console's writes for this lane (plus the shared power-up ones)
            // are the voice's writes.
            assert!(!v.writes.as_slice().is_empty(), "{kind:?}");
            assert_eq!(v.writes.as_slice(), cw.as_slice(), "{kind:?}");
        }
    }

    /// One studio pulse voice sounds like the same note alone on the console's
    /// Pulse 1: the same register writes, the same channel, the same DAC curve;
    /// only the frame counter's phase differs (each voice starts its own).
    #[test]
    fn a_lone_voice_matches_the_consoles_pulse() {
        let sr = 48_000.0;
        let n = Note { volume: Some(12), ..Note::new(0, 960 * 4, 69) };
        // Console.
        let mut syn = Box::new(Synth::new(sr));
        let samples = crate::samples::build(&mut syn.rom);
        let mut drv = Driver::new(samples);
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        let notes = [n];
        lanes[0] = &notes;
        let song = Song::plain(lanes);
        // Studio.
        let mut v = Voice::new(VoiceKind::Pulse, samples, 0);
        let mut blep = Box::new(Blep::new(CPU_HZ_NTSC, sr));
        let mut filt = crate::filter::OutputFilter::new(crate::filter::FilterMode::default(), sr);
        let rom = syn.rom;
        let (mut a, mut b) = (vec![0f32; 48_000], vec![0f32; 48_000]);
        let mut next_x2 = 0u64;
        for (ca, cb) in a.chunks_mut(512).zip(b.chunks_mut(512)) {
            let end = syn.cycle_for_samples(ca.len());
            while next_x2 / 2 < end {
                let t = next_x2 / 2;
                syn.run_until(t);
                v.run(t, &rom, &mut *blep);
                let mut w = Writes::default();
                let pos = t as f64 / CPU_HZ_NTSC * 2.5 * 960.0; // 150 BPM
                drv.tick(&song, Some(pos - 1.0), pos, true, &mut w);
                for &(ad, val) in w.as_slice() {
                    syn.write(ad, val);
                }
                let want = (pos < n.end as f64).then_some(n);
                v.tick(&song, want, crate::instrument::NONE, true, &drv.params.values[0], 0, &rom, &mut *blep);
                next_x2 += CPU_CYCLES_PER_FRAME_X2;
            }
            syn.render(ca);
            v.run(end, &rom, &mut *blep);
            blep.read(cb, end);
            for x in cb.iter_mut() {
                *x = filt.process(*x);
            }
        }
        let rms = |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        let (ra, rb) = (rms(&a[2000..]), rms(&b[2000..]));
        assert!(ra > 0.01, "{ra}");
        // Same level within 0.1 dB.
        std::println!("one pulse note: console RMS {ra:.5}, studio RMS {rb:.5}, {:+.3} dB", 20.0 * (rb / ra).log10());
        assert!((20.0 * (rb / ra).log10()).abs() < 0.1, "console {ra} vs studio {rb}");
    }

    /// Two pulses at once: the console's non-linear mixer makes them quieter
    /// than the sum of two lone pulses; studio voices add linearly. The
    /// measured difference goes into BATCH4A.md ("studio vs locked").
    #[test]
    fn two_voices_are_linear_two_console_pulses_are_not() {
        let one = mixer::pulse_out(15, 0);
        let two_console = mixer::pulse_out(15, 15);
        let two_studio = 2.0 * level_of(VoiceKind::Pulse, 15);
        let db = 20.0 * (two_studio / two_console).log10();
        std::println!("two pulses at volume 15 (peak level): console {two_console:.4}, studio {two_studio:.4}: studio {db:+.2} dB louder; one: {one:.4}");
        assert!(db > 0.5 && db < 1.5, "{db}");
    }

    /// Batch 5 A3: a studio voice is bipolar. Short notes on every voice type
    /// (a 12.5 % pulse, VRC6 pulse, saw, triangle, noise), raw (no output
    /// filter): the 50 ms running mean stays near 0 while notes start and stop
    /// (it swung by a duty x level before), and the output returns to exactly
    /// 0 when the voice is silent.
    #[test]
    fn studio_voices_are_bipolar() {
        use crate::instrument::NONE;
        let sr = 48_000.0;
        for kind in [VoiceKind::Pulse, VoiceKind::Vrc6Pulse, VoiceKind::Saw, VoiceKind::Triangle, VoiceKind::Noise] {
            let samples = crate::samples::build(&mut [0u8; ROM_SIZE]);
            let rom = [0u8; ROM_SIZE];
            let mut v = Voice::new(kind, samples, 0);
            let mut blep = Box::new(Blep::new(CPU_HZ_NTSC, sr));
            let mut params = crate::driver::Params::default().values[kind.lane()];
            // A thin pulse: the most lopsided waveform.
            params[crate::driver::ParamId::Duty as usize] = if kind == VoiceKind::Vrc6Pulse { 1 } else { 0 };
            // Four 150 ms notes, 100 ms apart (pitch 69, or the noise's own).
            let notes: std::vec::Vec<Note> = (0..4).map(|k| Note { volume: Some(15), ..Note::new(k * 600, k * 600 + 360, 69) }).collect();
            let song = Song::EMPTY;
            let mut out = vec![0f32; 48_000];
            let mut next_x2 = 0u64;

            for c in out.chunks_mut(480) {
                let end = blep.cycle_for(c.len());
                while next_x2 / 2 < end {
                    let t = next_x2 / 2;
                    v.run(t, &rom, &mut *blep);
                    let pos = t as f64 / CPU_HZ_NTSC * 2.5 * 960.0; // 150 BPM
                    let want = notes.iter().find(|n| (n.start as f64) <= pos && pos < n.end as f64).copied();
                    v.tick(&song, want, NONE, true, &params, 0, &rom, &mut *blep);
                    next_x2 += CPU_CYCLES_PER_FRAME_X2;
                }
                v.run(end, &rom, &mut *blep);
                blep.read(c, end);

            }
            let win = 2400;
            let (mut lo, mut hi) = (0f64, 0f64);
            let mut s: f64 = out[..win].iter().map(|&x| x as f64).sum();
            for i in win..out.len() {
                s += out[i] as f64 - out[i - win] as f64;
                lo = lo.min(s / win as f64);
                hi = hi.max(s / win as f64);
            }
            let peak = out.iter().fold(0f32, |m, x| m.max(x.abs())) as f64;
            std::println!("{kind:?}: peak {peak:.4}, 50 ms mean {lo:+.5} … {hi:+.5}");
            assert!(peak > 0.02, "{kind:?} sounds");
            // A pulse at 12.5 % used to swing by 1/8 of its level (peak/8 =
            // 0.0175 here); a bipolar voice stays within a few thousandths.
            // The noise: a fresh voice's LFSR starts from seed 1, whose first few
            // hundred clocks are mostly high (the hardware's own start-up
            // sequence, not an offset; unipolar, its swing was half its level,
            // 0.059 here).
            let lim = if kind == VoiceKind::Noise { 0.02 } else { 0.004 };
            assert!(lo > -lim && hi < lim, "{kind:?}: 50 ms mean {lo} … {hi}");
            assert!(v.silent(), "{kind:?}: silent after its notes");
        }
    }
}
