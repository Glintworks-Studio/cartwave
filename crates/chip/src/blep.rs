//! Band-limited step synthesis.
//!
//! The chip's mixed output is piecewise constant, changing at CPU-cycle
//! resolution (1.79 MHz). Each change of size `d` at time `t` is a step. We add
//! `d` times a band-limited impulse (windowed sinc) into an accumulation
//! buffer at `t`'s fractional output-sample position, and integrate the buffer
//! to get the output. That is the step convolved with a low-pass: no aliasing
//! from edges, and no smoothing beyond the chosen cutoff.
//!
//! Design (my own; see ENGINE.md for the measurements):
//! - Kernel: Kaiser-windowed sinc, `TAPS` = 32 output samples wide, cutoff
//!   `CUTOFF_RATIO` x fs, beta `KAISER_BETA`. Tabulated for `PHASES` = 256
//!   sub-sample positions (1/256 sample = 0.15 CPU cycle at 48 kHz).
//! - Exact integer arithmetic: each phase's taps are integers that sum to
//!   exactly `UNIT`, and amplitudes are fixed-point integers, so the
//!   integrator never drifts (a constant input reproduces exactly).
//! - Latency: `TAPS / 2` output samples (0.33 ms at 48 kHz).
//!
//! Cost: `TAPS` multiply-adds per output change, independent of the CPU clock.

use crate::math;

pub const TAPS: usize = 32;
pub const PHASE_BITS: u32 = 8;
pub const PHASES: usize = 1 << PHASE_BITS;
/// Tap scale: each phase's taps sum to this.
pub const UNIT_BITS: u32 = 15;
pub const UNIT: i32 = 1 << UNIT_BITS;
/// Fixed-point amplitude scale for `add`.
pub const AMP_BITS: u32 = 24;
/// Largest render chunk, in output samples.
pub const MAX_FRAMES: usize = 4096;
pub const CUTOFF_RATIO: f64 = 0.45;
pub const KAISER_BETA: f64 = 9.0;
/// Output delay introduced by the kernel, in samples.
pub const DELAY: usize = TAPS / 2;

const FRAC_BITS: u32 = 32;

pub struct Kernel {
    pub taps: [[i32; TAPS]; PHASES],
}

impl Kernel {
    pub fn new(cutoff_ratio: f64, beta: f64) -> Self {
        let mut taps = [[0i32; TAPS]; PHASES];
        let half = (TAPS / 2) as f64;
        let i0b = math::bessel_i0(beta);
        let fc2 = 2.0 * cutoff_ratio;
        for (ph, row) in taps.iter_mut().enumerate() {
            // Centre of the phase bin.
            let f = (ph as f64 + 0.5) / PHASES as f64;
            let mut vals = [0f64; TAPS];
            let mut sum = 0.0;
            for (j, v) in vals.iter_mut().enumerate() {
                let x = j as f64 - f - half; // distance from the step, minus the delay
                let r = x / half;
                let w = if math::abs(r) >= 1.0 {
                    0.0
                } else {
                    math::bessel_i0(beta * math::sqrt(1.0 - r * r)) / i0b
                };
                let a = math::PI * fc2 * x;
                let sinc = if math::abs(a) < 1e-12 { 1.0 } else { math::sin(a) / a };
                *v = fc2 * sinc * w;
                sum += *v;
            }
            // Normalise to exactly UNIT; put the rounding residue on the largest tap.
            let mut isum = 0i32;
            let mut big = 0;
            for j in 0..TAPS {
                row[j] = math::round(vals[j] / sum * UNIT as f64) as i32;
                isum += row[j];
                if row[j] > row[big] {
                    big = j;
                }
            }
            row[big] += UNIT - isum;
        }
        Self { taps }
    }
}

pub struct Blep {
    kernel: Kernel,
    buf: [i64; MAX_FRAMES + TAPS + 1],
    integ: i64,
    /// Output samples per CPU cycle, 32.32 fixed point.
    step: u64,
    /// CPU cycle at which `base_pos` was taken.
    base_cycle: u64,
    /// 32.32 output-sample position of `base_cycle`, relative to `buf[0]`.
    base_pos: u64,
    last_amp: i32,
    pub sample_rate: f64,
}

impl Blep {
    pub fn new(cpu_hz: f64, sample_rate: f64) -> Self {
        Self::with_kernel(cpu_hz, sample_rate, Kernel::new(CUTOFF_RATIO, KAISER_BETA))
    }

    pub fn with_kernel(cpu_hz: f64, sample_rate: f64, kernel: Kernel) -> Self {
        Self {
            kernel,
            buf: [0; MAX_FRAMES + TAPS + 1],
            integ: 0,
            step: math::round(sample_rate / cpu_hz * (1u64 << FRAC_BITS) as f64) as u64,
            base_cycle: 0,
            base_pos: 0,
            last_amp: 0,
            sample_rate,
        }
    }

    /// Starts at a steady level `amp` (as if it had always been there).
    pub fn preset(&mut self, amp: i32) {
        self.last_amp = amp;
        self.integ = amp as i64 * UNIT as i64;
    }

    #[inline]
    fn pos(&self, cycle: u64) -> u64 {
        self.base_pos + (cycle - self.base_cycle) * self.step
    }

    /// The amplitude becomes `amp` (fixed point, `AMP_BITS`) at `cycle`.
    /// `cycle` must not be before the last `end_frame` cycle, and must be
    /// before `cycle_for(MAX_FRAMES)`.
    #[inline]
    pub fn set(&mut self, cycle: u64, amp: i32) {
        let d = amp - self.last_amp;
        if d == 0 {
            return;
        }
        self.last_amp = amp;
        let p = self.pos(cycle);
        let i = (p >> FRAC_BITS) as usize;
        let ph = ((p >> (FRAC_BITS - PHASE_BITS)) as usize) & (PHASES - 1);
        let k = &self.kernel.taps[ph];
        let d = d as i64;
        for (b, &t) in self.buf[i..i + TAPS].iter_mut().zip(k.iter()) {
            *b += d * t as i64;
        }
    }

    /// The first CPU cycle whose position is at or past output sample `n`.
    /// Running the chip up to (excluding) this cycle completes `n` samples.
    pub fn cycle_for(&self, n: usize) -> u64 {
        let target = (n as u64) << FRAC_BITS;
        if target <= self.base_pos {
            return self.base_cycle;
        }
        let need = target - self.base_pos;
        self.base_cycle + need.div_ceil(self.step)
    }

    /// Integrates and outputs `out.len()` samples (scaled so that `amp` =
    /// 1 << AMP_BITS reads 1.0), then rebases at `now` (the next cycle to emulate,
    /// which must be `cycle_for(out.len())`).
    pub fn read(&mut self, out: &mut [f32], now: u64) {
        let n = out.len();
        debug_assert!(n <= MAX_FRAMES);
        let scale = 1.0 / ((1u64 << (AMP_BITS + UNIT_BITS)) as f64);
        for (o, b) in out.iter_mut().zip(self.buf[..n].iter()) {
            self.integ += *b;
            *o = (self.integ as f64 * scale) as f32;
        }
        self.buf.copy_within(n..n + TAPS, 0);
        self.buf[TAPS..n + TAPS].fill(0);
        let p = self.pos(now);
        self.base_pos = p - ((n as u64) << FRAC_BITS);
        self.base_cycle = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::CPU_HZ_NTSC;
    use std::boxed::Box;
    use std::vec;

    #[test]
    fn phases_sum_to_unit() {
        let k = Kernel::new(CUTOFF_RATIO, KAISER_BETA);
        for row in k.taps.iter() {
            assert_eq!(row.iter().sum::<i32>(), UNIT);
        }
    }

    #[test]
    fn step_settles_exactly_and_never_drifts() {
        let mut b = Box::new(Blep::new(CPU_HZ_NTSC, 48_000.0));
        let amp = 1 << AMP_BITS;
        let mut out = vec![0f32; 1000];
        // Toggle every 1001 cycles for many buffers; then hold.
        let mut cycle = 0u64;
        let mut level = 0;
        for _ in 0..200 {
            let end = b.cycle_for(out.len());
            while cycle + 1001 < end {
                cycle += 1001;
                level = if level == 0 { amp } else { 0 };
                b.set(cycle, level);
            }
            b.read(&mut out, end);
        }
        b.set(b.cycle_for(0).max(cycle + 1), amp);
        for _ in 0..3 {
            let end = b.cycle_for(out.len());
            b.read(&mut out, end);
        }
        // After settling, a held level reads exactly 1.0 (integer integrator).
        assert_eq!(out[999], 1.0);
        assert_eq!(out[0], 1.0);
    }

    #[test]
    fn step_position_and_delay() {
        let mut b = Box::new(Blep::new(CPU_HZ_NTSC, 48_000.0));
        // A step at exactly sample 100 should cross 0.5 at 100 + DELAY.
        let cyc = (100.0 * CPU_HZ_NTSC / 48_000.0) as u64;
        b.set(cyc, 1 << AMP_BITS);
        let mut out = vec![0f32; 300];
        let end = b.cycle_for(300);
        b.read(&mut out, end);
        let cross = out.iter().position(|&v| v >= 0.5).unwrap();
        assert!((cross as i64 - (100 + DELAY) as i64).abs() <= 1, "{cross}");
        // Overshoot (Gibbs) is bounded.
        let max = out.iter().cloned().fold(0f32, f32::max);
        assert!(max < 1.12 && max > 1.0, "{max}");
    }
}
