//! 2A03 noise channel. NESdev "APU Noise".

use crate::tables::NOISE_PERIOD_NTSC;
use crate::units::{catch_up_timer, Envelope, LengthCounter};

#[derive(Clone, Copy, Debug)]
pub struct Noise {
    /// 15-bit LFSR; loaded with 1 at power-up.
    pub shift: u16,
    /// M: periodic ("short", 93/31-step) mode.
    pub mode: bool,
    pub period_index: u8,
    pub timer_left: u32,
    pub env: Envelope,
    pub len: LengthCounter,
}

impl Default for Noise {
    fn default() -> Self {
        Self {
            shift: 1,
            mode: false,
            period_index: 0,
            timer_left: 2,
            env: Envelope::default(),
            len: LengthCounter::default(),
        }
    }
}

impl Noise {
    /// `reg` is 0..=3 for $400C-$400F.
    pub fn write(&mut self, reg: u16, v: u8) {
        match reg {
            0 => {
                self.len.halt = v & 0x20 != 0;
                self.env.write_control(v);
            }
            1 => {}
            2 => {
                self.mode = v & 0x80 != 0;
                self.period_index = v & 0x0F;
            }
            _ => {
                self.len.load(v >> 3);
                self.env.start = true;
            }
        }
    }

    /// One LFSR step: feedback = bit0 XOR (bit6 if M else bit1); shift right;
    /// feedback into bit 14.
    #[inline]
    pub fn step_lfsr(shift: u16, mode: bool) -> u16 {
        let other = if mode { 6 } else { 1 };
        let fb = (shift ^ (shift >> other)) & 1;
        (shift >> 1) | (fb << 14)
    }

    /// Returns whether bit 0 changed.
    #[inline]
    pub fn clock_timer(&mut self) -> bool {
        self.timer_left = NOISE_PERIOD_NTSC[self.period_index as usize] as u32;
        let old = self.shift;
        self.shift = Self::step_lfsr(old, self.mode);
        (old ^ self.shift) & 1 != 0
    }

    pub fn can_sleep(&self) -> bool {
        !self.len.active() || self.env.output() == 0
    }

    /// Advances the timer and the LFSR by `elapsed` cycles. The LFSR is
    /// stepped once per timer clock (at most ~1900 steps between frame clocks).
    pub fn catch_up(&mut self, elapsed: u64) {
        let p = NOISE_PERIOD_NTSC[self.period_index as usize] as u64;
        let n = catch_up_timer(&mut self.timer_left, p, elapsed);
        for _ in 0..n {
            self.shift = Self::step_lfsr(self.shift, self.mode);
        }
    }

    /// Silent while bit 0 of the LFSR is set or the length counter is 0.
    #[inline]
    pub fn output(&self) -> u8 {
        if self.shift & 1 != 0 || !self.len.active() {
            0
        } else {
            self.env.output()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::vec::Vec;

    fn period_of(mut s: u16, mode: bool) -> usize {
        let start = s;
        for i in 1..=40000 {
            s = Noise::step_lfsr(s, mode);
            if s == start {
                return i;
            }
        }
        0
    }

    #[test]
    fn long_mode_is_32767_steps() {
        assert_eq!(period_of(1, false), 32767);
    }

    #[test]
    fn short_mode_is_93_or_31_steps() {
        // Every state reachable in long mode falls into a 93- or 31-step loop in short mode.
        let mut lens = HashSet::new();
        let mut s = 1u16;
        for _ in 0..32767 {
            lens.insert(period_of(s, true));
            s = Noise::step_lfsr(s, false);
        }
        let mut lens: Vec<usize> = lens.into_iter().collect();
        lens.sort();
        assert_eq!(lens, [31, 93]);
    }

    #[test]
    fn first_outputs_from_power_up() {
        // From shift = 1 in long mode: the first 20 bit-0 values after each clock.
        // Derived by hand from the feedback rule (bit0 ^ bit1 -> bit14):
        // the single 1 bit shifts out on the first clock and re-enters at bit 14.
        let mut s = 1u16;
        let mut bits = Vec::new();
        for _ in 0..20 {
            s = Noise::step_lfsr(s, false);
            bits.push(s & 1);
        }
        // After clock 1: s = 0x4000 (fb = 1^0). Bit 0 stays 0 until the
        // 0x4000 bit reaches position 0 (clock 15), with bit-1 feedback
        // filling in behind it.
        assert_eq!(&bits[..14], &[0; 14]);
        assert_eq!(bits[14], 1);
        // Independent check: the sequence matches a direct bit-level model.
        let mut reg = [0u8; 15];
        reg[0] = 1;
        let mut model = Vec::new();
        for _ in 0..20 {
            let fb = reg[0] ^ reg[1];
            reg.rotate_left(1);
            reg[14] = fb;
            model.push(reg[0] as u16);
        }
        assert_eq!(bits, model);
    }

    #[test]
    fn period_table_and_muting() {
        let mut n = Noise::default();
        n.len.set_enabled(true);
        n.write(0, 0x30 | 9);
        n.write(2, 0x0A);
        n.write(3, 0x08);
        n.clock_timer();
        assert_eq!(n.timer_left, 380);
        n.shift = 0b10;
        assert_eq!(n.output(), 9);
        n.shift = 0b11;
        assert_eq!(n.output(), 0, "bit 0 set mutes");
        n.shift = 0b10;
        n.len.set_enabled(false);
        assert_eq!(n.output(), 0, "length 0 mutes");
    }
}
