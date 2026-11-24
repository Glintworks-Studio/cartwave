//! 2A03 pulse channel. NESdev "APU Pulse" (incl. the sweep section).

use crate::tables::PULSE_DUTY;
use crate::units::{catch_up_timer, Envelope, LengthCounter};

#[derive(Clone, Copy, Debug)]
pub struct Pulse {
    /// Pulse 1 negates with ones' complement (-c - 1), pulse 2 with two's (-c).
    ones_complement: bool,
    pub duty: u8,
    /// Sequencer step, 0..8, in `PULSE_DUTY` output order.
    pub step: u8,
    /// 11-bit timer period t.
    pub period: u16,
    /// CPU cycles until the next timer clock of the sequencer.
    pub timer_left: u32,
    pub env: Envelope,
    pub len: LengthCounter,
    sweep_enabled: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_reload: bool,
    sweep_divider: u8,
}

impl Pulse {
    pub fn new(ones_complement: bool) -> Self {
        Self {
            ones_complement,
            duty: 0,
            step: 0,
            period: 0,
            timer_left: 2,
            env: Envelope::default(),
            len: LengthCounter::default(),
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            sweep_divider: 0,
        }
    }

    /// `reg` is 0..=3 for $4000-$4003 (or $4004-$4007).
    pub fn write(&mut self, reg: u16, v: u8) {
        match reg {
            0 => {
                self.duty = v >> 6;
                self.len.halt = v & 0x20 != 0;
                self.env.write_control(v);
            }
            1 => {
                self.sweep_enabled = v & 0x80 != 0;
                self.sweep_period = (v >> 4) & 7;
                self.sweep_negate = v & 0x08 != 0;
                self.sweep_shift = v & 7;
                self.sweep_reload = true;
            }
            2 => self.period = (self.period & 0x700) | v as u16,
            _ => {
                self.period = (self.period & 0xFF) | (((v & 7) as u16) << 8);
                self.len.load(v >> 3);
                self.env.start = true;
                self.step = 0;
            }
        }
    }

    /// The timer reached zero: advance the sequencer and reload.
    /// Period t gives t + 1 APU cycles = 2(t + 1) CPU cycles per step.
    #[inline]
    pub fn clock_timer(&mut self) {
        self.step = (self.step + 1) & 7;
        self.timer_left = 2 * (self.period as u32 + 1);
    }

    /// Output is 0 whatever the sequencer does, until a register write or a
    /// frame-counter clock changes something.
    pub fn can_sleep(&self) -> bool {
        !self.len.active() || self.sweep_muted() || self.env.output() == 0
    }

    /// Advances the timer and sequencer by `elapsed` cycles in closed form.
    pub fn catch_up(&mut self, elapsed: u64) {
        let n = catch_up_timer(&mut self.timer_left, 2 * (self.period as u64 + 1), elapsed);
        self.step = ((self.step as u64 + n) & 7) as u8;
    }

    pub fn target_period(&self) -> u32 {
        let p = self.period as i32;
        let c = p >> self.sweep_shift;
        let t = if self.sweep_negate {
            p - c - self.ones_complement as i32
        } else {
            p + c
        };
        t.max(0) as u32
    }

    /// Muted by the sweep unit: t < 8, or the target period overflows $7FF.
    /// This applies whether or not the sweep is enabled.
    pub fn sweep_muted(&self) -> bool {
        self.period < 8 || self.target_period() > 0x7FF
    }

    /// Half-frame clock: length counter and sweep.
    pub fn clock_half(&mut self) {
        self.len.clock();
        if self.sweep_divider == 0
            && self.sweep_enabled
            && self.sweep_shift > 0
            && !self.sweep_muted()
        {
            self.period = self.target_period() as u16;
        }
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    #[inline]
    pub fn output(&self) -> u8 {
        if PULSE_DUTY[self.duty as usize][self.step as usize] == 0
            || !self.len.active()
            || self.sweep_muted()
        {
            0
        } else {
            self.env.output()
        }
    }

    pub fn frequency(cpu_hz: f64, period: u16) -> f64 {
        cpu_hz / (16.0 * (period as f64 + 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::CPU_HZ_NTSC;
    use std::vec::Vec;

    fn on(p: &mut Pulse, duty: u8, period: u16) {
        p.len.set_enabled(true);
        p.write(0, (duty << 6) | 0x30 | 15); // halt length, constant volume 15
        p.write(1, 0x08); // sweep off, negate (cannot overflow)
        p.write(2, period as u8);
        p.write(3, (1 << 3) | (period >> 8) as u8);
    }

    /// Runs `cycles` CPU cycles and returns the output after each cycle.
    fn run(p: &mut Pulse, cycles: u32) -> Vec<u8> {
        (0..cycles)
            .map(|_| {
                p.timer_left -= 1;
                if p.timer_left == 0 {
                    p.clock_timer();
                }
                p.output()
            })
            .collect()
    }

    #[test]
    fn duty_waveforms_and_period() {
        for duty in 0..4u8 {
            let mut p = Pulse::new(false);
            let t = 100u16;
            on(&mut p, duty, t);
            p.timer_left = 2 * (t as u32 + 1);
            let out = run(&mut p, 16 * (t as u32 + 1) * 3);
            // One full waveform is 16(t+1) CPU cycles; high share = duty table.
            let per = 16 * (t as usize + 1);
            let high = out[per..2 * per].iter().filter(|&&v| v > 0).count();
            assert_eq!(high, per * [1, 2, 4, 6][duty as usize] / 8, "duty {duty}");
            assert_eq!(&out[per..2 * per], &out[2 * per..3 * per]);
        }
        // f = CPU / (16 (t + 1)): t = 253 -> 440.6 Hz (A4 ≈ +2.5 cents).
        let f = Pulse::frequency(CPU_HZ_NTSC, 253);
        assert!((f - 440.39).abs() < 0.05, "{f}");
    }

    #[test]
    fn write_4003_resets_phase_and_envelope() {
        let mut p = Pulse::new(false);
        on(&mut p, 2, 50);
        run(&mut p, 333);
        assert_ne!(p.step, 0);
        p.write(3, 0x08);
        assert_eq!(p.step, 0);
        assert!(p.env.start);
    }

    #[test]
    fn sweep_mute_thresholds() {
        let mut p = Pulse::new(false);
        on(&mut p, 2, 8);
        assert!(!p.sweep_muted());
        p.period = 7;
        assert!(p.sweep_muted(), "t < 8 mutes");
        // Sweep disabled, add mode, shift 0: target = 2t; overflow mutes anyway.
        p.write(1, 0x00);
        p.period = 0x3FF;
        assert!(!p.sweep_muted(), "0x3FF + 0x3FF = 0x7FE fits");
        p.period = 0x400;
        assert!(p.sweep_muted(), "0x400 + 0x400 > 0x7FF mutes although sweep is disabled");
        p.write(1, 0x01); // shift 1: 0x400 + 0x200 = 0x600 fits
        assert!(!p.sweep_muted());
        p.write(1, 0x08); // negate: never overflows
        p.period = 0x7FF;
        assert!(!p.sweep_muted());
    }

    #[test]
    fn negate_ones_vs_twos_complement() {
        let mut p1 = Pulse::new(true);
        let mut p2 = Pulse::new(false);
        for p in [&mut p1, &mut p2] {
            p.period = 0x100;
            p.write(1, 0x08 | 2); // negate, shift 2: change = 0x40
        }
        assert_eq!(p1.target_period(), 0x100 - 0x40 - 1);
        assert_eq!(p2.target_period(), 0x100 - 0x40);
    }

    #[test]
    fn sweep_updates_period_on_half_frames() {
        let mut p = Pulse::new(false);
        on(&mut p, 2, 0x100);
        p.write(1, 0x80 | (1 << 4) | 0x08 | 3); // enabled, P = 1, negate, shift 3
        // First half clock: reload flag set, divider 0 -> update and reload.
        p.clock_half();
        assert_eq!(p.period, 0x100 - 0x20);
        p.clock_half(); // divider 1 -> 0, no update
        assert_eq!(p.period, 0x100 - 0x20);
        p.clock_half(); // divider 0 -> update
        assert_eq!(p.period, 0xE0 - 0x1C);
    }
}
