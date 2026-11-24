//! The console's analog output stage, as first-order filters.
//!
//! NESdev "APU Mixer", current revision (27 Oct 2026): NES = first-order
//! high-pass at 90 Hz + first-order low-pass at 14 kHz (+ a second-order
//! low-pass far above the audio band, not modelled). Revisions from 2014 to
//! 2026 also listed "another first-order high-pass filter at 440 Hz"; it came
//! from a curve fit to recordings, no component in the schematic matches it,
//! and it was removed. We default to the current two-stage version and keep
//! the three-stage version selectable for A/B. Famicom: a single first-order
//! high-pass at 37 Hz (RF modulator not modelled). See ENGINE.md.
//!
//! Each stage is the bilinear transform of a first-order RC section with the
//! cutoff pre-warped, so the -3 dB point is exact at the output rate.

use crate::math;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilterMode {
    /// Only a 5 Hz DC blocker (keeps speakers safe; inaudible).
    Off,
    /// NES front-loader: HP 90 Hz, LP 14 kHz (current wiki).
    #[default]
    Nes,
    /// NES with the extra HP 440 Hz stage listed on the wiki 2014-2026.
    NesWith440,
    /// Famicom: HP 37 Hz only.
    Famicom,
}

impl FilterMode {
    pub const ALL: [FilterMode; 4] = [Self::Off, Self::Nes, Self::NesWith440, Self::Famicom];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Nes => "NES",
            Self::NesWith440 => "NES+440",
            Self::Famicom => "FC",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OnePole {
    b0: f32,
    b1: f32,
    a1: f32,
    x1: f32,
    y1: f32,
}

impl OnePole {
    pub fn lowpass(fc: f64, fs: f64) -> Self {
        let k = math::tan(math::PI * fc / fs);
        let b0 = k / (1.0 + k);
        Self {
            b0: b0 as f32,
            b1: b0 as f32,
            a1: ((k - 1.0) / (k + 1.0)) as f32,
            ..Self::default()
        }
    }

    pub fn highpass(fc: f64, fs: f64) -> Self {
        let k = math::tan(math::PI * fc / fs);
        let b0 = 1.0 / (1.0 + k);
        Self {
            b0: b0 as f32,
            b1: -b0 as f32,
            a1: ((k - 1.0) / (k + 1.0)) as f32,
            ..Self::default()
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 - self.a1 * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }

    /// Steady state for a constant input `x` held forever.
    pub fn settle(&mut self, x: f32) -> f32 {
        // DC gain: low-pass 1, high-pass 0.
        let y = if self.b1 > 0.0 { x } else { 0.0 };
        self.x1 = x;
        self.y1 = y;
        y
    }

    /// Magnitude response at `f` (for tests).
    pub fn gain_at(&self, f: f64, fs: f64) -> f64 {
        // |H(e^jw)| for H = (b0 + b1 z^-1) / (1 + a1 z^-1)
        let w = 2.0 * math::PI * f / fs;
        let (c, s) = (math::cos(w), math::sin(w));
        let (b0, b1, a1) = (self.b0 as f64, self.b1 as f64, self.a1 as f64);
        let nr = b0 + b1 * c;
        let ni = -b1 * s;
        let dr = 1.0 + a1 * c;
        let di = -a1 * s;
        math::sqrt((nr * nr + ni * ni) / (dr * dr + di * di))
    }
}

pub const MAX_STAGES: usize = 3;

#[derive(Clone, Copy, Debug)]
pub struct OutputFilter {
    pub mode: FilterMode,
    stages: [OnePole; MAX_STAGES],
    count: usize,
}

impl OutputFilter {
    pub fn new(mode: FilterMode, fs: f64) -> Self {
        let mut stages = [OnePole::default(); MAX_STAGES];
        let list: &[OnePole] = match mode {
            FilterMode::Off => &[OnePole::highpass(5.0, fs)],
            FilterMode::Nes => &[OnePole::highpass(90.0, fs), OnePole::lowpass(14_000.0, fs)],
            FilterMode::NesWith440 => &[
                OnePole::highpass(90.0, fs),
                OnePole::highpass(440.0, fs),
                OnePole::lowpass(14_000.0, fs),
            ],
            FilterMode::Famicom => &[OnePole::highpass(37.0, fs)],
        };
        stages[..list.len()].copy_from_slice(list);
        Self {
            mode,
            stages,
            count: list.len(),
        }
    }

    #[inline]
    pub fn process(&mut self, mut x: f32) -> f32 {
        for s in &mut self.stages[..self.count] {
            x = s.process(x);
        }
        x
    }

    /// Steady state for a constant input held forever.
    pub fn settle(&mut self, mut x: f32) {
        for s in &mut self.stages[..self.count] {
            x = s.settle(x);
        }
    }

    pub fn gain_at(&self, f: f64, fs: f64) -> f64 {
        self.stages[..self.count]
            .iter()
            .map(|s| s.gain_at(f, fs))
            .product()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoffs_are_minus_3db() {
        let fs = 48_000.0;
        let lp = OnePole::lowpass(14_000.0, fs);
        let hp = OnePole::highpass(90.0, fs);
        let half = core::f64::consts::FRAC_1_SQRT_2;
        assert!((lp.gain_at(14_000.0, fs) - half).abs() < 1e-4);
        assert!((hp.gain_at(90.0, fs) - half).abs() < 1e-4);
        assert!(lp.gain_at(100.0, fs) > 0.9999);
        assert!(hp.gain_at(10_000.0, fs) > 0.9999);
    }

    #[test]
    fn nes_chain() {
        let fs = 48_000.0;
        let f = OutputFilter::new(FilterMode::Nes, fs);
        let db = |g: f64| 20.0 * g.log10();
        assert!(db(f.gain_at(1000.0, fs)) > -0.5);
        assert!(db(f.gain_at(30.0, fs)) < -9.0);
        let f3 = OutputFilter::new(FilterMode::NesWith440, fs);
        assert!(db(f3.gain_at(200.0, fs)) < -6.0, "440 Hz stage cuts bass hard");
        // DC is fully removed in every mode.
        for m in FilterMode::ALL {
            let mut f = OutputFilter::new(m, fs);
            let mut y = 0.0;
            for _ in 0..fs as usize * 2 {
                y = f.process(1.0);
            }
            assert!(y.abs() < 1e-3, "{m:?} {y}");
        }
    }
}
