//! 2A03 triangle channel. NESdev "APU Triangle".
//!
//! Ultrasonic periods (t = 0 or 1, 55.9 / 28.0 kHz) are emulated exactly like
//! any other period: the sequencer steps every t + 1 CPU cycles. The
//! band-limited synthesis then removes everything above the audio band, which
//! leaves the waveform's average (~7.5, "roughly half amplitude", as the wiki
//! describes for real hardware's external filtering). Because every step is
//! band-limited, nothing aliases back down. Many players instead mute such
//! periods; we don't, so the documented pop when a game writes period 0 to
//! silence the channel is preserved. The cost is up to one event per CPU cycle
//! while it lasts (measured in ENGINE.md).

use crate::tables::TRIANGLE_SEQ;
use crate::units::{catch_up_timer, LengthCounter};

#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub step: u8,
    pub period: u16,
    pub timer_left: u32,
    pub len: LengthCounter,
    /// C: linear counter control (also the length counter halt flag).
    control: bool,
    reload_value: u8,
    pub linear: u8,
    reload_flag: bool,
}

impl Default for Triangle {
    fn default() -> Self {
        Self {
            step: 0,
            period: 0,
            timer_left: 1,
            len: LengthCounter::default(),
            control: false,
            reload_value: 0,
            linear: 0,
            reload_flag: false,
        }
    }
}

impl Triangle {
    /// `reg` is 0..=3 for $4008-$400B.
    pub fn write(&mut self, reg: u16, v: u8) {
        match reg {
            0 => {
                self.control = v & 0x80 != 0;
                self.len.halt = self.control;
                self.reload_value = v & 0x7F;
            }
            1 => {}
            2 => self.period = (self.period & 0x700) | v as u16,
            _ => {
                self.period = (self.period & 0xFF) | (((v & 7) as u16) << 8);
                self.len.load(v >> 3);
                self.reload_flag = true;
            }
        }
    }

    /// Timer clocked every CPU cycle; period t gives t + 1 cycles per step.
    /// Returns whether the output may have changed.
    #[inline]
    pub fn clock_timer(&mut self) -> bool {
        self.timer_left = self.period as u32 + 1;
        if self.linear > 0 && self.len.active() {
            self.step = (self.step + 1) & 31;
            true
        } else {
            false
        }
    }

    /// The sequencer is gated: the output holds until a write or frame clock.
    pub fn can_sleep(&self) -> bool {
        self.linear == 0 || !self.len.active()
    }

    /// Advances the timer by `elapsed` cycles while gated (no steps).
    pub fn catch_up(&mut self, elapsed: u64) {
        catch_up_timer(&mut self.timer_left, self.period as u64 + 1, elapsed);
    }

    /// Quarter-frame clock: the linear counter.
    pub fn clock_quarter(&mut self) {
        if self.reload_flag {
            self.linear = self.reload_value;
        } else if self.linear > 0 {
            self.linear -= 1;
        }
        if !self.control {
            self.reload_flag = false;
        }
    }

    /// The triangle has no volume control; when the sequencer is gated it holds
    /// its current step's value.
    #[inline]
    pub fn output(&self) -> u8 {
        TRIANGLE_SEQ[self.step as usize]
    }

    pub fn frequency(cpu_hz: f64, period: u16) -> f64 {
        cpu_hz / (32.0 * (period as f64 + 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::CPU_HZ_NTSC;
    use std::vec::Vec;

    fn start(t: &mut Triangle, period: u16) {
        t.len.set_enabled(true);
        t.write(0, 0xFF); // control set, reload 127
        t.write(2, period as u8);
        t.write(3, 0x08 | (period >> 8) as u8);
        t.clock_quarter(); // loads the linear counter
    }

    #[test]
    fn sequence_and_period() {
        let mut t = Triangle::default();
        start(&mut t, 3);
        t.timer_left = 4;
        let mut out = Vec::new();
        for _ in 0..(32 * 4) {
            t.timer_left -= 1;
            if t.timer_left == 0 {
                t.clock_timer();
            }
            out.push(t.output());
        }
        // Steps every t+1 = 4 cycles (on cycle 4, 8, ...), through the
        // documented 32-step sequence; out[0] is still step 0.
        let steps: Vec<u8> = out.iter().step_by(4).copied().collect();
        assert_eq!(steps, TRIANGLE_SEQ);
        assert_eq!(out[2], TRIANGLE_SEQ[0]);
        assert_eq!(out[3], TRIANGLE_SEQ[1]);
        // f = CPU / (32 (t+1)): t = 126 -> 440.4 Hz.
        assert!((Triangle::frequency(CPU_HZ_NTSC, 126) - 440.39).abs() < 0.05);
    }

    #[test]
    fn linear_counter_rules() {
        let mut t = Triangle::default();
        t.len.set_enabled(true);
        t.write(0, 0x05); // control clear, reload 5
        t.write(3, 0x08);
        t.clock_quarter();
        assert_eq!(t.linear, 5, "reload flag reloads");
        t.clock_quarter();
        assert_eq!(t.linear, 4, "control clear: reload flag was cleared, counter decrements");
        for _ in 0..10 {
            t.clock_quarter();
        }
        assert_eq!(t.linear, 0);
        let before = t.step;
        t.clock_timer();
        assert_eq!(t.step, before, "linear counter 0 gates the sequencer");

        // Control set keeps the reload flag set: every clock reloads.
        t.write(0, 0x80 | 9);
        t.write(3, 0x08);
        for _ in 0..5 {
            t.clock_quarter();
            assert_eq!(t.linear, 9);
        }
        // Halt method 1: write $80 -> reloads 0 at the next quarter frame.
        t.write(0, 0x80);
        t.clock_quarter();
        assert_eq!(t.linear, 0);
    }

    #[test]
    fn length_counter_gates() {
        let mut t = Triangle::default();
        start(&mut t, 10);
        t.len.set_enabled(false);
        let before = t.step;
        t.clock_timer();
        assert_eq!(t.step, before);
    }
}
