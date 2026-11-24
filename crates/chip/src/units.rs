//! Shared APU sub-units: envelope generator and length counter.

use crate::tables::LENGTH_TABLE;

/// Envelope generator (pulse and noise). NESdev "APU Envelope".
#[derive(Clone, Copy, Debug, Default)]
pub struct Envelope {
    pub start: bool,
    pub loop_flag: bool,
    pub constant: bool,
    /// V: constant volume and divider reload value.
    pub volume: u8,
    divider: u8,
    decay: u8,
}

impl Envelope {
    /// Bits 5-0 of $4000/$4004/$400C (`..LC VVVV`).
    pub fn write_control(&mut self, v: u8) {
        self.loop_flag = v & 0x20 != 0;
        self.constant = v & 0x10 != 0;
        self.volume = v & 0x0F;
    }

    /// Quarter-frame clock.
    pub fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay = 15;
            self.divider = self.volume;
        } else if self.divider == 0 {
            self.divider = self.volume;
            if self.decay > 0 {
                self.decay -= 1;
            } else if self.loop_flag {
                self.decay = 15;
            }
        } else {
            self.divider -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        if self.constant {
            self.volume
        } else {
            self.decay
        }
    }
}

/// Length counter (pulse, triangle, noise). NESdev "APU Length Counter".
#[derive(Clone, Copy, Debug, Default)]
pub struct LengthCounter {
    pub enabled: bool,
    pub halt: bool,
    pub count: u8,
}

impl LengthCounter {
    /// Load from bits 7-3 of the fourth channel register; ignored while disabled.
    pub fn load(&mut self, index: u8) {
        if self.enabled {
            self.count = LENGTH_TABLE[(index & 0x1F) as usize];
        }
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.count = 0;
        }
    }

    /// Half-frame clock.
    pub fn clock(&mut self) {
        if self.count > 0 && !self.halt {
            self.count -= 1;
        }
    }

    pub fn active(&self) -> bool {
        self.count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decay from 15 to 0 takes 15 x (V+1) quarter frames after the start clock.
    #[test]
    fn envelope_decay_timing() {
        for v in [0u8, 3, 15] {
            let mut e = Envelope::default();
            e.write_control(v); // decay mode, no loop
            e.start = true;
            e.clock(); // start: decay = 15, divider = V
            assert_eq!(e.output(), 15);
            let mut clocks = 0;
            while e.output() > 0 {
                e.clock();
                clocks += 1;
            }
            assert_eq!(clocks, 15 * (v as u32 + 1), "V={v}");
            // Stays at 0 without loop.
            for _ in 0..100 {
                e.clock();
            }
            assert_eq!(e.output(), 0);
        }
    }

    #[test]
    fn envelope_loop_and_constant() {
        let mut e = Envelope::default();
        e.write_control(0x20); // loop, V = 0: one decay step per quarter frame
        e.start = true;
        e.clock();
        for _ in 0..15 {
            e.clock();
        }
        assert_eq!(e.output(), 0);
        e.clock();
        assert_eq!(e.output(), 15, "loop reloads 15");
        // Constant volume overrides the output but the decay keeps running.
        e.write_control(0x10 | 7);
        assert_eq!(e.output(), 7);
    }

    #[test]
    fn length_counter_rules() {
        let mut l = LengthCounter::default();
        l.load(1);
        assert_eq!(l.count, 0, "disabled channels ignore loads");
        l.set_enabled(true);
        l.load(1);
        assert_eq!(l.count, 254);
        l.halt = true;
        l.clock();
        assert_eq!(l.count, 254, "halt freezes");
        l.halt = false;
        l.clock();
        assert_eq!(l.count, 253);
        l.set_enabled(false);
        assert_eq!(l.count, 0, "disable clears");
    }
}
