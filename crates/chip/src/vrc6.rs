//! VRC6 expansion audio: two pulses and a sawtooth. NESdev "VRC6 audio".
//!
//! Register addresses are the VRC6a layout ($x000-$x003). On VRC6b boards
//! address lines A0/A1 are swapped ($x001 <-> $x002); that's a cartridge
//! wiring detail with no effect on the sound, so we only expose VRC6a.

#[derive(Clone, Copy, Debug)]
pub struct Vrc6Pulse {
    pub volume: u8,
    /// D: 0..=7, high for D + 1 of 16 steps.
    pub duty: u8,
    /// M: "digitized" mode, ignore the duty generator and output V constantly.
    pub mode: bool,
    pub period: u16,
    pub enabled: bool,
    /// Duty generator step, counts down 15..0.
    pub step: u8,
    pub timer_left: u32,
}

impl Default for Vrc6Pulse {
    fn default() -> Self {
        Self {
            volume: 0,
            duty: 0,
            mode: false,
            period: 0,
            enabled: false,
            step: 15,
            timer_left: 1,
        }
    }
}

impl Vrc6Pulse {
    pub fn write(&mut self, reg: u16, v: u8) {
        match reg {
            0 => {
                self.mode = v & 0x80 != 0;
                self.duty = (v >> 4) & 7;
                self.volume = v & 0x0F;
            }
            1 => self.period = (self.period & 0xF00) | v as u16,
            _ => {
                self.period = (self.period & 0xFF) | (((v & 0x0F) as u16) << 8);
                let en = v & 0x80 != 0;
                if !en {
                    // Output forced to 0; the duty generator is reset and halted.
                    self.step = 15;
                }
                self.enabled = en;
            }
        }
    }

    /// Divider underflow (period >> shift) + 1 CPU cycles. Returns whether the
    /// output may have changed.
    #[inline]
    pub fn clock_timer(&mut self, shift: u8) -> bool {
        self.timer_left = (self.period >> shift) as u32 + 1;
        if self.enabled {
            self.step = if self.step == 0 { 15 } else { self.step - 1 };
            !self.mode
        } else {
            false
        }
    }

    #[inline]
    pub fn output(&self) -> u8 {
        if self.enabled && (self.mode || self.step <= self.duty) {
            self.volume
        } else {
            0
        }
    }

    pub fn frequency(cpu_hz: f64, period: u16) -> f64 {
        cpu_hz / (16.0 * (period as f64 + 1.0))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vrc6Saw {
    /// A: accumulator rate, 6 bits. Above 42 the accumulator wraps (distortion).
    pub rate: u8,
    pub period: u16,
    pub enabled: bool,
    pub acc: u8,
    /// Number of adds since the last reset; the 7th accumulator clock resets.
    adds: u8,
    /// The accumulator reacts to every second divider clock.
    odd: bool,
    pub timer_left: u32,
}

impl Default for Vrc6Saw {
    fn default() -> Self {
        Self {
            rate: 0,
            period: 0,
            enabled: false,
            acc: 0,
            adds: 0,
            odd: false,
            timer_left: 1,
        }
    }
}

impl Vrc6Saw {
    pub fn write(&mut self, reg: u16, v: u8) {
        match reg {
            0 => self.rate = v & 0x3F,
            1 => self.period = (self.period & 0xF00) | v as u16,
            2 => {
                self.period = (self.period & 0xFF) | (((v & 0x0F) as u16) << 8);
                self.enabled = v & 0x80 != 0;
                if !self.enabled {
                    // Held at 0 until E is set; the divider is not reset.
                    self.acc = 0;
                    self.adds = 0;
                }
            }
            _ => {}
        }
    }

    #[inline]
    pub fn clock_timer(&mut self, shift: u8) -> bool {
        self.timer_left = (self.period >> shift) as u32 + 1;
        self.odd = !self.odd;
        if self.odd || !self.enabled {
            return false;
        }
        let old = self.acc >> 3;
        if self.adds == 6 {
            self.acc = 0;
            self.adds = 0;
        } else {
            self.acc = self.acc.wrapping_add(self.rate);
            self.adds += 1;
        }
        old != self.acc >> 3
    }

    /// High 5 bits of the accumulator.
    #[inline]
    pub fn output(&self) -> u8 {
        if self.enabled {
            self.acc >> 3
        } else {
            0
        }
    }

    pub fn frequency(cpu_hz: f64, period: u16) -> f64 {
        cpu_hz / (14.0 * (period as f64 + 1.0))
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vrc6 {
    pub pulse1: Vrc6Pulse,
    pub pulse2: Vrc6Pulse,
    pub saw: Vrc6Saw,
    /// $9003 H: halts all oscillators in their current state.
    pub halt: bool,
    /// Right shift applied to all periods: 0, 4 (16x) or 8 (256x; overrides 16x).
    pub shift: u8,
}

impl Vrc6 {
    /// Register write; `addr` in $9000-$9003, $A000-$A002, $B000-$B002.
    pub fn write(&mut self, addr: u16, v: u8) {
        let reg = addr & 3;
        match addr & 0xF000 {
            0x9000 if reg == 3 => {
                self.halt = v & 1 != 0;
                self.shift = if v & 4 != 0 {
                    8
                } else if v & 2 != 0 {
                    4
                } else {
                    0
                };
            }
            0x9000 => self.pulse1.write(reg, v),
            0xA000 => self.pulse2.write(reg, v),
            0xB000 => self.saw.write(reg, v),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::CPU_HZ_NTSC;
    use std::vec::Vec;

    #[test]
    fn saw_accumulator_matches_wiki_trace() {
        // Wiki example, A = $08: outputs per divider step 0..15.
        let mut s = Vrc6Saw::default();
        s.write(0, 0x08);
        s.write(2, 0x80);
        let mut out = std::vec![s.output()];
        for _ in 0..16 {
            s.clock_timer(0);
            out.push(s.output());
        }
        assert_eq!(out, [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 0, 0, 1]);
    }

    #[test]
    fn saw_wraps_above_42() {
        let mut s = Vrc6Saw::default();
        s.write(0, 43);
        s.write(2, 0x80);
        let mut accs = Vec::new();
        for _ in 0..14 {
            s.clock_timer(0);
            accs.push(s.acc);
        }
        assert_eq!(accs[11], (43u32 * 6 % 256) as u8, "6 x 43 = 258 wraps to 2");
        let mut s = Vrc6Saw::default();
        s.write(0, 42);
        s.write(2, 0x80);
        for _ in 0..12 {
            s.clock_timer(0);
        }
        assert_eq!(s.acc, 252, "42 is the largest rate that doesn't wrap");
        assert_eq!(s.output(), 31);
    }

    #[test]
    fn pulse_duty_settings() {
        for d in 0..8u8 {
            let mut p = Vrc6Pulse::default();
            p.write(0, (d << 4) | 0x0F);
            p.write(2, 0x80);
            let high = (0..16)
                .filter(|_| {
                    p.clock_timer(0);
                    p.output() > 0
                })
                .count();
            assert_eq!(high, d as usize + 1, "duty {d}");
        }
        let mut p = Vrc6Pulse::default();
        p.write(0, 0x80 | 0x0A);
        p.write(2, 0x80);
        assert!((0..16).all(|_| {
            p.clock_timer(0);
            p.output() == 10
        }));
        p.write(2, 0x00);
        assert_eq!(p.output(), 0, "disable forces 0");
        assert_eq!(p.step, 15, "and resets the duty generator");
    }

    #[test]
    fn frequency_and_shift() {
        // Pulse: f = CPU / (16 (t+1)); saw: CPU / (14 (t+1)).
        assert!((Vrc6Pulse::frequency(CPU_HZ_NTSC, 253) - 440.39).abs() < 0.05);
        assert!((Vrc6Saw::frequency(CPU_HZ_NTSC, 289) - 440.83).abs() < 0.05);
        let mut v = Vrc6::default();
        v.write(0x9003, 0x02);
        assert_eq!(v.shift, 4);
        v.write(0x9003, 0x06);
        assert_eq!(v.shift, 8, "256x overrides 16x");
        v.pulse1.period = 0x123;
        v.pulse1.clock_timer(v.shift);
        assert_eq!(v.pulse1.timer_left, 0x01 + 1);
        v.write(0x9003, 0x01);
        assert!(v.halt);
    }
}
