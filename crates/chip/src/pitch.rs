//! MIDI note -> period register, per oscillator, with the tuning error.
//!
//! Period formulas (NESdev): pulse f = CPU / (16 (t+1)), triangle
//! f = CPU / (32 (t+1)), VRC6 pulse f = CPU / (16 (t+1)), VRC6 saw
//! f = CPU / (14 (t+1)). Equal temperament, A4 = 440 Hz. The period is the
//! nearest integer; the error is what remains.

use crate::math;
use crate::tables::CPU_HZ_NTSC;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Osc {
    Pulse,
    Triangle,
    Vrc6Pulse,
    Saw,
}

impl Osc {
    pub fn divisor(self) -> f64 {
        match self {
            Osc::Pulse | Osc::Vrc6Pulse => 16.0,
            Osc::Triangle => 32.0,
            Osc::Saw => 14.0,
        }
    }

    /// Usable period range. Pulse periods below 8 are muted by the sweep unit.
    pub fn period_range(self) -> (u16, u16) {
        match self {
            Osc::Pulse => (8, 0x7FF),
            Osc::Triangle => (0, 0x7FF),
            Osc::Vrc6Pulse | Osc::Saw => (0, 0xFFF),
        }
    }

    pub fn frequency(self, period: u16) -> f64 {
        CPU_HZ_NTSC / (self.divisor() * (period as f64 + 1.0))
    }
}

pub fn midi_hz(note: f64) -> f64 {
    440.0 * math::exp2((note - 69.0) / 12.0)
}

/// Nearest period for `note`, or `None` if it is outside the oscillator's range.
pub fn period_for(osc: Osc, note: u8) -> Option<u16> {
    let t = CPU_HZ_NTSC / (osc.divisor() * midi_hz(note as f64)) - 1.0;
    let t = math::round(t);
    let (lo, hi) = osc.period_range();
    if t < lo as f64 || t > hi as f64 {
        None
    } else {
        Some(t as u16)
    }
}

/// Signed tuning error of `period` against `note`, in cents.
pub fn cents_error(osc: Osc, note: u8, period: u16) -> f64 {
    1200.0 * math::log2(osc.frequency(period) / midi_hz(note as f64))
}

/// The noise channel has no pitch register: map a note to one of the 16
/// period indices. Higher notes -> shorter periods. Index = (96 - note) / 4,
/// clamped, so the demo's kick (40) -> 14, snare (60) -> 9, hat (76) -> 5.
pub fn noise_index_for(note: u8) -> u8 {
    ((96i32 - note as i32) / 4).clamp(0, 15) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a440() {
        assert_eq!(period_for(Osc::Pulse, 69), Some(253));
        assert_eq!(period_for(Osc::Triangle, 69), Some(126));
        // 440.397 Hz: +1.56 cents.
        let e = cents_error(Osc::Pulse, 69, 253);
        assert!((e - 1.5607).abs() < 0.001, "{e}");
    }

    #[test]
    fn ranges() {
        // Pulse lowest note: t <= 0x7FF -> f >= 54.6 Hz (A1 = 55 Hz).
        assert_eq!(period_for(Osc::Pulse, 32), None);
        assert!(period_for(Osc::Pulse, 33).is_some());
        // Triangle reaches an octave lower.
        assert!(period_for(Osc::Triangle, 21).is_some());
        // VRC6's 12-bit periods reach another octave below the 2A03 pulse.
        assert!(period_for(Osc::Vrc6Pulse, 21).is_some());
    }

    /// The error never exceeds half a period step, so it shrinks with pitch
    /// going down. Within the demo's range (C2..C7) every oscillator is within
    /// +/-12 cents (measured: see ENGINE.md for the full table).
    #[test]
    fn errors_bounded() {
        for osc in [Osc::Pulse, Osc::Triangle, Osc::Vrc6Pulse, Osc::Saw] {
            for note in 36..=96u8 {
                if let Some(t) = period_for(osc, note) {
                    let e = cents_error(osc, note, t);
                    // Half a step of period t is 1200 log2((t+1.5)/(t+1)) cents.
                    let bound = 1200.0 * ((t as f64 + 1.5) / (t as f64 + 1.0)).log2();
                    assert!(e.abs() <= bound + 1e-9, "{osc:?} {note}: {e}");
                }
            }
        }
    }

    #[test]
    fn noise_mapping() {
        assert_eq!(noise_index_for(40), 14);
        assert_eq!(noise_index_for(60), 9);
        assert_eq!(noise_index_for(76), 5);
        assert_eq!(noise_index_for(127), 0);
        assert_eq!(noise_index_for(0), 15);
    }
}
