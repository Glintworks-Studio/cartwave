//! Automation envelopes as the engine plays them (Batch 5, VISION.md
//! "Automation"): breakpoints in song ticks, evaluated **once per frame
//! tick** by the driver (locked tracks: quantised to the register's real
//! steps, as game sound drivers did) and by studio voices.
//!
//! - A point holds its value in the target's own units (a register value, a
//!   macro step, cents, dB) and the **curve** of the segment that starts at
//!   it (-1 … 1, 0 = straight; `shape`).
//! - Before the first point the envelope holds the first value, after the
//!   last the last (Live's envelopes do the same). Two points on one tick
//!   make a jump.
//! - Pure arithmetic (add, multiply, divide, compare): the same value on
//!   every machine. No allocation: envelopes are slices of the song
//!   snapshot's flat point list.

use crate::driver::ParamId;

/// One breakpoint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoPoint {
    /// Song position (ticks, 960 per quarter note).
    pub tick: u32,
    /// The value in the target's units.
    pub value: f32,
    /// The curve of the segment from this point to the next: 0 straight,
    /// > 0 rises early (convex), < 0 late (concave). -1 … 1.
    pub curve: f32,
}

/// What an envelope drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AutoTarget {
    /// A channel control (register value, quantised).
    Chip(ParamId),
    /// The volume macro's step for the frame (0 … 15).
    MacroVolume,
    /// The arpeggio offset (semitones).
    MacroArp,
    /// The fine pitch (cents), in place of the pitch macro.
    MacroPitch,
    /// The vibrato depth (cents).
    MacroVibrato,
    /// The duty macro's step (sixteenths).
    MacroDuty,
    /// The triangle's gate (0 off, 1 on).
    MacroGate,
    /// A studio track's fader (dB; -70 and below is silence).
    Gain,
    /// A studio track's pan (-50 … 50).
    Pan,
    /// A studio track's activator (0 off, 1 on).
    Active,
    /// Studio voices: unison voices, detune and drift (cents).
    Unison,
    Detune,
    Drift,
    /// The Main track's gain (dB).
    MainGain,
}

/// One envelope: its target and its points (`points[start .. start + len]`
/// of the snapshot's point list).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoEnv {
    pub target: AutoTarget,
    pub start: u32,
    pub len: u32,
}

impl AutoEnv {
    pub fn points<'a>(&self, all: &'a [AutoPoint]) -> &'a [AutoPoint] {
        let s = self.start as usize;
        all.get(s..s + self.len as usize).unwrap_or(&[])
    }
}

/// The segment shape: `x` (0 … 1 across the segment) bent by `curve`
/// (-1 … 1). A rational bias curve, `x / (x + (1 - x) k)` with
/// `k = (1 - c) / (1 + c)`: straight at 0, symmetric for ±c, no `pow`.
pub fn shape(x: f32, curve: f32) -> f32 {
    let c = curve.clamp(-0.98, 0.98);
    if c == 0.0 {
        return x;
    }
    let k = (1.0 - c) / (1.0 + c);
    let d = x + (1.0 - x) * k;
    if d <= 0.0 {
        0.0
    } else {
        x / d
    }
}

/// The envelope's value at song position `pos` (ticks).
pub fn eval(points: &[AutoPoint], pos: f64) -> Option<f32> {
    let first = points.first()?;
    // The last point at or before `pos`.
    let i = points.partition_point(|p| (p.tick as f64) <= pos);
    if i == 0 {
        return Some(first.value);
    }
    let a = points[i - 1];
    let Some(b) = points.get(i) else { return Some(a.value) };
    let span = b.tick as f64 - a.tick as f64;
    if span <= 0.0 {
        return Some(b.value);
    }
    let x = ((pos - a.tick as f64) / span) as f32;
    Some(a.value + (b.value - a.value) * shape(x.clamp(0.0, 1.0), a.curve))
}

/// A register-style value: the envelope rounded to the nearest step and
/// clamped into `lo ..= hi` (the chip's real resolution).
pub fn quantise(v: f32, lo: i32, hi: i32) -> i32 {
    let r = if v >= 0.0 { (v + 0.5) as i32 } else { -((-v + 0.5) as i32) };
    r.clamp(lo, hi)
}

/// Linear gain for a fader value in dB (-70 dB and below: silence).
pub fn db_to_gain(db: f32) -> f32 {
    if db <= -70.0 {
        0.0
    } else {
        crate::math::exp(db as f64 * core::f64::consts::LN_10 / 20.0) as f32
    }
}

/// The instrument macros a frame's automation replaces on one lane
/// (`None`: the instrument's own macro plays).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MacroOvr {
    pub volume: Option<u8>,
    pub arp: Option<i8>,
    /// Cents.
    pub pitch: Option<i16>,
    /// Vibrato depth, cents.
    pub vibrato: Option<u8>,
    /// Sixteenths.
    pub duty: Option<u8>,
    pub gate: Option<bool>,
}

impl MacroOvr {
    pub fn any(&self) -> bool {
        *self != Self::default()
    }

    /// Sets the macro `t` from envelope value `v` (quantised to its steps);
    /// returns false for a target that isn't a macro.
    pub fn set(&mut self, t: AutoTarget, v: f32) -> bool {
        match t {
            AutoTarget::MacroVolume => self.volume = Some(quantise(v, 0, 15) as u8),
            AutoTarget::MacroArp => self.arp = Some(quantise(v, -12, 12) as i8),
            AutoTarget::MacroPitch => self.pitch = Some(quantise(v, -100, 100) as i16),
            AutoTarget::MacroVibrato => self.vibrato = Some(quantise(v, 0, 100) as u8),
            AutoTarget::MacroDuty => self.duty = Some(quantise(v, 0, 15) as u8),
            AutoTarget::MacroGate => self.gate = Some(quantise(v, 0, 1) != 0),
            _ => return false,
        }
        true
    }
}

/// One lane's automation for the frame: channel controls (mask bit per
/// `ParamId`) and macros.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LaneAuto {
    pub mask: u16,
    pub vals: [i8; ParamId::ALL.len()],
    pub mac: MacroOvr,
}

impl LaneAuto {
    /// Evaluates `envs` (one lane's) at `pos` for driver lane `lane`:
    /// channel controls quantised to their range, macros to their steps.
    /// Targets that aren't channel controls or macros are left to the caller.
    pub fn eval(envs: &[AutoEnv], points: &[AutoPoint], lane: usize, pos: f64) -> Self {
        let mut a = Self::default();
        for e in envs {
            let Some(v) = eval(e.points(points), pos) else { continue };
            match e.target {
                AutoTarget::Chip(p) => {
                    let (lo, hi, _) = p.range(lane);
                    a.mask |= 1 << p as u16;
                    a.vals[p as usize] = quantise(v, lo, hi) as i8;
                }
                t => {
                    a.mac.set(t, v);
                }
            }
        }
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(v: &[(u32, f32, f32)]) -> std::vec::Vec<AutoPoint> {
        v.iter().map(|&(tick, value, curve)| AutoPoint { tick, value, curve }).collect()
    }

    #[test]
    fn eval_holds_interpolates_and_jumps() {
        let p = pts(&[(960, 0.0, 0.0), (1920, 10.0, 0.0), (1920, 4.0, 0.0), (2880, 4.0, 0.0)]);
        assert_eq!(eval(&[], 0.0), None);
        assert_eq!(eval(&p, 0.0), Some(0.0), "before the first point: its value");
        assert_eq!(eval(&p, 1440.0), Some(5.0), "halfway");
        assert_eq!(eval(&p, 1920.0), Some(4.0), "a jump: the later point wins at its tick");
        assert_eq!(eval(&p, 9999.0), Some(4.0), "after the last: its value");
    }

    #[test]
    fn curves_bend_symmetrically_and_keep_the_ends() {
        for c in [-0.9f32, -0.5, 0.3, 0.9] {
            assert_eq!(shape(0.0, c), 0.0);
            assert!((shape(1.0, c) - 1.0).abs() < 1e-6);
            // Mirror symmetry: shape(x, c) = 1 - shape(1 - x, -c).
            for x in [0.1f32, 0.25, 0.5, 0.8] {
                assert!((shape(x, c) - (1.0 - shape(1.0 - x, -c))).abs() < 1e-5, "{x} {c}");
            }
        }
        assert!(shape(0.5, 0.6) > 0.5 && shape(0.5, -0.6) < 0.5);
        assert_eq!(shape(0.37, 0.0), 0.37);
    }

    #[test]
    fn quantise_rounds_to_register_steps() {
        assert_eq!(quantise(7.49, 0, 15), 7);
        assert_eq!(quantise(7.5, 0, 15), 8);
        assert_eq!(quantise(-1.5, -7, 7), -2);
        assert_eq!(quantise(99.0, 0, 3), 3);
        assert_eq!(db_to_gain(0.0), 1.0);
        assert_eq!(db_to_gain(-80.0), 0.0);
        assert!((db_to_gain(-6.0) - 0.501).abs() < 0.001);
    }

    #[test]
    fn a_lane_frame_quantises_controls_and_macros() {
        let points = pts(&[(0, 0.0, 0.0), (960, 15.0, 0.0), (0, 3.0, 0.0), (0, 2.6, 0.0)]);
        let envs = [
            AutoEnv { target: AutoTarget::Chip(ParamId::Volume), start: 0, len: 2 },
            AutoEnv { target: AutoTarget::Chip(ParamId::Duty), start: 2, len: 1 },
            AutoEnv { target: AutoTarget::MacroArp, start: 3, len: 1 },
        ];
        let a = LaneAuto::eval(&envs, &points, 0, 480.0);
        assert_eq!(a.mask, (1 << ParamId::Volume as u16) | (1 << ParamId::Duty as u16));
        assert_eq!(a.vals[ParamId::Volume as usize], 8, "7.5 rounds up");
        assert_eq!(a.vals[ParamId::Duty as usize], 3);
        assert_eq!(a.mac.arp, Some(3));
    }
}
