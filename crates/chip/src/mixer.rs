//! Output mixing. NESdev "APU Mixer" (2A03) and "VRC6 audio" (expansion).
//!
//! 2A03: the documented non-linear formulas, evaluated exactly (only when an
//! output changes, so the divisions cost nothing measurable).
//!
//! VRC6: the wiki gives a linear 6-bit DAC summing the two 4-bit pulses and the
//! 5-bit saw, and says "at maximum volume, the pulse channels of the VRC6 are
//! roughly equivalent to the pulse channels of the 2A03 (except inverted)".
//! That is the only documented level claim we found (ENGINE.md lists the
//! sources checked), so one VRC6 DAC step = pulse_out(15) / 15 and the VRC6
//! sum is subtracted (inverted polarity).

/// Channel DAC inputs, in lane order: pulse1, pulse2, triangle, noise, dmc,
/// vrc6 pulse1, vrc6 pulse2, saw.
pub type Outputs = [u8; 8];

/// Exact 2A03 pulse group: 95.88 / (8128 / (p1 + p2) + 100), 0 when silent.
pub fn pulse_out(p1: u8, p2: u8) -> f64 {
    let n = (p1 + p2) as f64;
    if n == 0.0 {
        0.0
    } else {
        95.88 / (8128.0 / n + 100.0)
    }
}

/// Exact 2A03 triangle/noise/DMC group.
pub fn tnd_out(t: u8, n: u8, d: u8) -> f64 {
    let s = t as f64 / 8227.0 + n as f64 / 12241.0 + d as f64 / 22638.0;
    if s == 0.0 {
        0.0
    } else {
        159.79 / (1.0 / s + 100.0)
    }
}

/// The wiki's lookup-table approximation (31 + 203 entries), for comparison.
pub fn pulse_table(n: usize) -> f64 {
    if n == 0 {
        0.0
    } else {
        95.52 / (8128.0 / n as f64 + 100.0)
    }
}

pub fn tnd_table(n: usize) -> f64 {
    if n == 0 {
        0.0
    } else {
        163.67 / (24329.0 / n as f64 + 100.0)
    }
}

/// One VRC6 DAC step: a 2A03 pulse at volume 15, divided by 15.
pub fn vrc6_step() -> f64 {
    pulse_out(15, 0) / 15.0
}

/// VRC6 polarity relative to the 2A03 ("except inverted").
pub const VRC6_SIGN: f64 = -1.0;

/// Full mix, nominally 0..~1 for the 2A03 part.
pub fn mix(o: &Outputs, vrc6_step: f64) -> f64 {
    let apu = pulse_out(o[0], o[1]) + tnd_out(o[2], o[3], o[4]);
    let vrc6 = (o[5] as f64 + o[6] as f64 + o[7] as f64) * vrc6_step;
    apu + VRC6_SIGN * vrc6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_values() {
        // Hand-computed from the formulas.
        assert!((pulse_out(15, 15) - 95.88 / (8128.0 / 30.0 + 100.0)).abs() < 1e-12);
        assert!((pulse_out(15, 15) - 0.258_483).abs() < 1e-5);
        assert!((pulse_out(15, 0) - 0.149_377).abs() < 1e-5);
        assert_eq!(pulse_out(0, 0), 0.0);
        assert_eq!(tnd_out(0, 0, 0), 0.0);
        // All maxed: 159.79 / (1 / (15/8227 + 15/12241 + 127/22638) + 100)
        let s = 15.0 / 8227.0 + 15.0 / 12241.0 + 127.0 / 22638.0;
        assert!((tnd_out(15, 15, 127) - 159.79 / (1.0 / s + 100.0)).abs() < 1e-12);
        assert!((tnd_out(15, 15, 127) - 0.741_516).abs() < 1e-5);
        // Full-scale 2A03 output is just under 1.0.
        assert!(pulse_out(15, 15) + tnd_out(15, 15, 127) < 1.001);
    }

    #[test]
    fn nonlinear_not_additive() {
        // Two pulses at 15 are quieter than twice one pulse at 15.
        assert!(pulse_out(15, 15) < 2.0 * pulse_out(15, 0) * 0.9);
    }

    #[test]
    fn lookup_tables_match_formulas() {
        // Pulse table: within 0.4% (numerator 95.52 vs 95.88).
        for n in 1..=30usize {
            let exact = 95.88 / (8128.0 / n as f64 + 100.0);
            let rel = (pulse_table(n) - exact).abs() / exact;
            assert!(rel < 0.004, "pulse {n}: {rel}");
        }
        // tnd table indexed 3t + 2n + d. The wiki says "within 4%". Measured:
        // every combination is within 4% except the DMC alone (t = n = 0,
        // d = 1..=26), which reads up to 4.66% louder: the documented
        // "slightly louder DMC" of a DMC-based unit.
        let mut worst: f64 = 0.0;
        for t in 0..16u8 {
            for n in 0..16u8 {
                for d in 0..128u8 {
                    let exact = tnd_out(t, n, d);
                    if exact == 0.0 {
                        continue;
                    }
                    let approx = tnd_table(3 * t as usize + 2 * n as usize + d as usize);
                    let rel = (approx - exact).abs() / exact;
                    worst = worst.max(rel);
                    if rel >= 0.04 {
                        assert!(t == 0 && n == 0 && d <= 26, "({t},{n},{d}): {rel}");
                    }
                }
            }
        }
        assert!(worst < 0.0467, "worst tnd table error {worst}");
    }

    #[test]
    fn vrc6_level() {
        let o: Outputs = [0, 0, 0, 0, 0, 15, 0, 0];
        assert!((mix(&o, vrc6_step()) + pulse_out(15, 0)).abs() < 1e-12);
        let saw: Outputs = [0, 0, 0, 0, 0, 0, 0, 31];
        assert!((mix(&saw, vrc6_step()).abs() - 31.0 / 15.0 * pulse_out(15, 0)).abs() < 1e-12);
    }
}
