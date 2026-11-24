//! The whole sound hardware (2A03 APU + optional VRC6), clocked event by
//! event, feeding the band-limited step buffer and the analog output filter.
//!
//! Event-driven: instead of stepping every CPU cycle, `run_until` jumps to the
//! next cycle where any timer or the frame counter fires. Between events no
//! output can change. The result is identical to per-cycle stepping (tested).

use crate::blep::{Blep, AMP_BITS, MAX_FRAMES};
use crate::dmc::{Dmc, ROM_SIZE};
use crate::filter::{FilterMode, OutputFilter};
use crate::frame_counter::FrameCounter;
use crate::mixer::{self, Outputs};
use crate::noise::Noise;
use crate::pulse::Pulse;
use crate::tables::CPU_HZ_NTSC;
use crate::triangle::Triangle;
use crate::vrc6::Vrc6;

pub struct Synth {
    pub pulse1: Pulse,
    pub pulse2: Pulse,
    pub triangle: Triangle,
    pub noise: Noise,
    pub dmc: Dmc,
    pub frame: FrameCounter,
    pub vrc6: Vrc6,
    /// Whether the VRC6 is present (2A03 only otherwise).
    pub vrc6_on: bool,
    pub rom: [u8; ROM_SIZE],
    /// CPU cycles emulated so far.
    pub cycle: u64,
    outputs: Outputs,
    vrc6_step: f64,
    blep: Blep,
    filter: OutputFilter,
    /// Per-channel peak DAC input since the last `take_peaks`.
    peaks: Outputs,
    /// DMC level range since the last `take_peaks` (its level is a DC offset).
    dmc_min: u8,
    dmc_max: u8,
    /// Output changes fed to the step buffer (cost metric).
    pub events: u64,
    /// When false, `run_until` point-samples instead of band-limiting
    /// (the naive reference for the aliasing measurement).
    pub band_limited: bool,
    naive_amp: f32,
}

impl Synth {
    pub fn new(sample_rate: f64) -> Self {
        let mut s = Self {
            pulse1: Pulse::new(true),
            pulse2: Pulse::new(false),
            triangle: Triangle::default(),
            noise: Noise::default(),
            dmc: Dmc::default(),
            frame: FrameCounter::default(),
            vrc6: Vrc6::default(),
            vrc6_on: true,
            rom: [0; ROM_SIZE],
            cycle: 0,
            outputs: [0; 8],
            vrc6_step: mixer::vrc6_step(),
            blep: Blep::new(CPU_HZ_NTSC, sample_rate),
            filter: OutputFilter::new(FilterMode::default(), sample_rate),
            peaks: [0; 8],
            dmc_min: 0,
            dmc_max: 0,
            events: 0,
            band_limited: true,
            naive_amp: 0.0,
        };
        // Power-up levels (the triangle rests on step 0 = 15) are treated as
        // having always been there: no click when the engine starts.
        s.outputs = s.current_outputs();
        let amp = mixer::mix(&s.outputs, s.vrc6_step);
        s.blep.preset(Self::fixed(amp));
        s.filter.settle(Self::quantized(amp));
        s
    }

    fn fixed(amp: f64) -> i32 {
        (amp * (1u64 << AMP_BITS) as f64) as i32
    }

    /// The value the step buffer outputs for a held `amp`.
    fn quantized(amp: f64) -> f32 {
        (Self::fixed(amp) as f64 / (1u64 << AMP_BITS) as f64) as f32
    }

    pub fn sample_rate(&self) -> f64 {
        self.blep.sample_rate
    }

    pub fn set_filter(&mut self, mode: FilterMode) {
        if mode != self.filter.mode {
            self.filter = OutputFilter::new(mode, self.blep.sample_rate);
            self.filter.settle(Self::quantized(mixer::mix(&self.outputs, self.vrc6_step)));
        }
    }

    pub fn filter_mode(&self) -> FilterMode {
        self.filter.mode
    }

    /// CPU register write at the current cycle. $4000-$4017 and the VRC6 ranges.
    pub fn write(&mut self, addr: u16, v: u8) {
        match addr {
            0x4000..=0x4003 => self.pulse1.write(addr & 3, v),
            0x4004..=0x4007 => self.pulse2.write(addr & 3, v),
            0x4008..=0x400B => self.triangle.write(addr & 3, v),
            0x400C..=0x400F => self.noise.write(addr & 3, v),
            0x4010..=0x4013 => self.dmc.write(addr & 3, v),
            0x4015 => {
                self.pulse1.len.set_enabled(v & 1 != 0);
                self.pulse2.len.set_enabled(v & 2 != 0);
                self.triangle.len.set_enabled(v & 4 != 0);
                self.noise.len.set_enabled(v & 8 != 0);
                self.dmc.set_enabled(v & 0x10 != 0, &self.rom);
            }
            0x4017 => self.frame.write(v, self.cycle),
            0x9000..=0x9003 | 0xA000..=0xA002 | 0xB000..=0xB002 => self.vrc6.write(addr, v),
            _ => {}
        }
        self.refresh_output();
    }

    /// $4015 read (without the read side effect on the frame IRQ).
    pub fn status(&self) -> u8 {
        (self.pulse1.len.active() as u8)
            | (self.pulse2.len.active() as u8) << 1
            | (self.triangle.len.active() as u8) << 2
            | (self.noise.len.active() as u8) << 3
            | (self.dmc.active() as u8) << 4
            | (self.frame.irq as u8) << 6
            | (self.dmc.irq as u8) << 7
    }

    pub fn outputs(&self) -> Outputs {
        self.outputs
    }

    #[inline]
    fn current_outputs(&self) -> Outputs {
        [
            self.pulse1.output(),
            self.pulse2.output(),
            self.triangle.output(),
            self.noise.output(),
            self.dmc.output(),
            if self.vrc6_on { self.vrc6.pulse1.output() } else { 0 },
            if self.vrc6_on { self.vrc6.pulse2.output() } else { 0 },
            if self.vrc6_on { self.vrc6.saw.output() } else { 0 },
        ]
    }

    #[inline]
    fn refresh_output(&mut self) {
        let o = self.current_outputs();
        if o == self.outputs {
            return;
        }
        self.outputs = o;
        for (p, &v) in self.peaks.iter_mut().zip(o.iter()) {
            *p = (*p).max(v);
        }
        self.dmc_min = self.dmc_min.min(o[4]);
        self.dmc_max = self.dmc_max.max(o[4]);
        let amp = mixer::mix(&o, self.vrc6_step);
        if self.band_limited {
            self.events += 1;
            self.blep.set(self.cycle, Self::fixed(amp));
        } else {
            self.naive_amp = amp as f32;
        }
    }

    /// Emulates up to (excluding) CPU cycle `target`.
    pub fn run_until(&mut self, target: u64) {
        while self.cycle < target {
            let mut dt = (target - self.cycle).min(u32::MAX as u64) as u32;
            dt = dt
                .min(self.pulse1.timer_left)
                .min(self.pulse2.timer_left)
                .min(self.triangle.timer_left)
                .min(self.noise.timer_left)
                .min(self.dmc.timer_left)
                .min(self.frame.left());
            let vrc6_running = self.vrc6_on && !self.vrc6.halt;
            if vrc6_running {
                dt = dt
                    .min(self.vrc6.pulse1.timer_left)
                    .min(self.vrc6.pulse2.timer_left)
                    .min(self.vrc6.saw.timer_left);
            }
            self.cycle += dt as u64;
            let mut dirty = false;

            self.pulse1.timer_left -= dt;
            if self.pulse1.timer_left == 0 {
                self.pulse1.clock_timer();
                dirty = true;
            }
            self.pulse2.timer_left -= dt;
            if self.pulse2.timer_left == 0 {
                self.pulse2.clock_timer();
                dirty = true;
            }
            self.triangle.timer_left -= dt;
            if self.triangle.timer_left == 0 {
                dirty |= self.triangle.clock_timer();
            }
            self.noise.timer_left -= dt;
            if self.noise.timer_left == 0 {
                dirty |= self.noise.clock_timer();
            }
            self.dmc.timer_left -= dt;
            if self.dmc.timer_left == 0 {
                dirty |= self.dmc.clock_timer(&self.rom);
            }
            if vrc6_running {
                let sh = self.vrc6.shift;
                self.vrc6.pulse1.timer_left -= dt;
                if self.vrc6.pulse1.timer_left == 0 {
                    dirty |= self.vrc6.pulse1.clock_timer(sh);
                }
                self.vrc6.pulse2.timer_left -= dt;
                if self.vrc6.pulse2.timer_left == 0 {
                    dirty |= self.vrc6.pulse2.clock_timer(sh);
                }
                self.vrc6.saw.timer_left -= dt;
                if self.vrc6.saw.timer_left == 0 {
                    dirty |= self.vrc6.saw.clock_timer(sh);
                }
            }
            let c = self.frame.advance(dt);
            if c.quarter {
                self.pulse1.env.clock();
                self.pulse2.env.clock();
                self.noise.env.clock();
                self.triangle.clock_quarter();
                dirty = true;
            }
            if c.half {
                self.pulse1.clock_half();
                self.pulse2.clock_half();
                self.triangle.len.clock();
                self.noise.len.clock();
            }
            if dirty {
                self.refresh_output();
            }
        }
    }

    /// The CPU cycle at which `n` more output samples are complete.
    pub fn cycle_for_samples(&self, n: usize) -> u64 {
        self.blep.cycle_for(n)
    }

    /// Renders `out.len()` mono samples (<= `MAX_FRAMES`), emulating the chip
    /// up to the matching cycle. Register writes must be applied between
    /// `run_until` calls before this; see `Player` for frame-tick scheduling.
    pub fn render(&mut self, out: &mut [f32]) {
        debug_assert!(out.len() <= MAX_FRAMES);
        if self.band_limited {
            let end = self.blep.cycle_for(out.len());
            self.run_until(end);
            self.blep.read(out, end);
        } else {
            // Naive: point-sample the mixed output at each output sample time.
            for o in out.iter_mut() {
                let end = self.blep.cycle_for(1);
                self.run_until(end);
                self.blep.read(&mut [0.0], end);
                *o = self.naive_amp;
            }
        }
        for o in out.iter_mut() {
            *o = self.filter.process(*o);
        }
    }

    /// Per-channel activity since the last call, 0..1, in lane order.
    pub fn take_peaks(&mut self) -> [f32; 8] {
        let mut r = [0f32; 8];
        for i in 0..8 {
            let full = match i {
                4 => 127.0,
                7 => 31.0,
                _ => 15.0,
            };
            r[i] = self.peaks[i] as f32 / full;
        }
        // DMC: its level is a DC offset; show the swing instead.
        r[4] = (self.dmc_max - self.dmc_min.min(self.dmc_max)) as f32 / 64.0;
        self.peaks = self.outputs;
        self.dmc_min = self.outputs[4];
        self.dmc_max = self.outputs[4];
        r.map(|v| v.min(1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::boxed::Box;
    use std::vec;
    use std::vec::Vec;

    fn setup(s: &mut Synth) {
        s.write(0x4015, 0x1F);
        s.write(0x4017, 0x00);
        // Pulse 1: duty 2, decaying envelope, sweep up; pulse 2: constant.
        s.write(0x4000, 0x80 | 0x05);
        s.write(0x4001, 0x80 | 0x20 | 0x02);
        s.write(0x4002, 0x40);
        s.write(0x4003, 0x01 | 0x08);
        s.write(0x4004, 0x70 | 0x0A);
        s.write(0x4006, 0x99);
        s.write(0x4007, 0x08);
        s.write(0x4008, 0x81);
        s.write(0x400A, 0x30);
        s.write(0x400B, 0x08);
        s.write(0x400C, 0x3F);
        s.write(0x400E, 0x03);
        s.write(0x400F, 0x08);
        s.write(0x9000, 0x3F);
        s.write(0x9001, 0x20);
        s.write(0x9002, 0x81);
        s.write(0xB000, 0x10);
        s.write(0xB001, 0x60);
        s.write(0xB002, 0x80);
    }

    /// Event-driven stepping must match stepping one cycle at a time.
    #[test]
    fn event_driven_equals_per_cycle() {
        let mut a = Box::new(Synth::new(48_000.0));
        let mut b = Box::new(Synth::new(48_000.0));
        // Compare channel states only; don't feed the step buffer.
        a.band_limited = false;
        b.band_limited = false;
        setup(&mut a);
        setup(&mut b);
        let n = 200_000u64;
        let mut trace_a = Vec::new();
        let mut trace_b = Vec::new();
        // a: big jumps, recording outputs at every 997th cycle.
        let mut t = 0;
        while t < n {
            t += 997;
            a.run_until(t);
            trace_a.push(a.outputs());
        }
        let mut t = 0;
        while t < n {
            for _ in 0..997 {
                b.run_until(b.cycle + 1);
            }
            t += 997;
            trace_b.push(b.outputs());
        }
        assert_eq!(trace_a, trace_b);
        assert_eq!(a.status(), b.status());
    }

    #[test]
    fn silent_at_power_up() {
        let mut s = Box::new(Synth::new(48_000.0));
        let mut out = vec![0f32; 4096];
        s.render(&mut out);
        assert!(out.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn renders_audio() {
        let mut s = Box::new(Synth::new(48_000.0));
        setup(&mut s);
        let mut out = vec![0f32; 4096];
        s.render(&mut out);
        let rms = (out.iter().map(|v| v * v).sum::<f32>() / out.len() as f32).sqrt();
        assert!(rms > 0.01, "{rms}");
        assert!(out.iter().all(|v| v.abs() < 2.0));
    }
}
