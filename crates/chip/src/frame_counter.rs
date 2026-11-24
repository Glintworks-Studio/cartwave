//! APU frame counter (sequencer). NESdev "APU Frame Counter".
//!
//! Event-driven: `left()` is the number of CPU cycles until the next thing it
//! does (a step or a pending reset).

use crate::tables::{FRAME_PERIOD_4, FRAME_PERIOD_5, FRAME_STEPS_4, FRAME_STEPS_5};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Clocks {
    pub quarter: bool,
    pub half: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct FrameCounter {
    pub five_step: bool,
    pub irq_inhibit: bool,
    pub irq: bool,
    /// Index of the next step.
    step: usize,
    /// Cycles until that step.
    step_left: u32,
    /// Pending $4017 write: (cycles until it takes effect, value).
    pending: Option<(u32, u8)>,
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self {
            five_step: false,
            irq_inhibit: false,
            irq: false,
            step: 0,
            step_left: FRAME_STEPS_4[0],
            pending: None,
        }
    }
}

impl FrameCounter {
    pub fn left(&self) -> u32 {
        match self.pending {
            Some((d, _)) => d.min(self.step_left),
            None => self.step_left,
        }
    }

    /// $4017 write. The reset happens 3 CPU cycles later if the write lands on
    /// an APU cycle (we define those as even CPU cycles, where the pulse/noise/
    /// DMC timers clock), 4 if it lands between. Bit 6 (IRQ inhibit) acts
    /// immediately.
    pub fn write(&mut self, v: u8, cpu_cycle: u64) {
        self.irq_inhibit = v & 0x40 != 0;
        if self.irq_inhibit {
            self.irq = false;
        }
        let delay = if cpu_cycle.is_multiple_of(2) { 3 } else { 4 };
        self.pending = Some((delay, v));
    }

    fn steps(&self) -> (&'static [u32], u32) {
        if self.five_step {
            (&FRAME_STEPS_5, FRAME_PERIOD_5)
        } else {
            (&FRAME_STEPS_4, FRAME_PERIOD_4)
        }
    }

    /// Advances `dt` cycles (never past `left()`), returning the clocks to apply.
    pub fn advance(&mut self, dt: u32) -> Clocks {
        debug_assert!(dt <= self.left());
        let mut c = Clocks::default();
        let mut reset_now = false;
        if let Some((d, v)) = &mut self.pending {
            *d -= dt;
            if *d == 0 {
                reset_now = true;
                self.five_step = *v & 0x80 != 0;
            }
        }
        self.step_left -= dt;
        if reset_now {
            self.pending = None;
            self.step = 0;
            self.step_left = FRAME_STEPS_4[0]; // same first step in both modes
            if self.five_step {
                // Writing with M set also clocks quarter and half frame immediately.
                c.quarter = true;
                c.half = true;
            }
            return c;
        }
        if self.step_left == 0 {
            let (steps, period) = self.steps();
            let at = steps[self.step];
            match (self.five_step, self.step) {
                (_, 0) | (_, 2) => c.quarter = true,
                (_, 1) => {
                    c.quarter = true;
                    c.half = true;
                }
                (false, 3) => {
                    c.quarter = true;
                    c.half = true;
                    if !self.irq_inhibit {
                        self.irq = true;
                    }
                }
                (true, 3) => {}
                _ => {
                    c.quarter = true;
                    c.half = true;
                }
            }
            self.step += 1;
            if self.step == steps.len() {
                self.step = 0;
                self.step_left = period - at + steps[0];
            } else {
                self.step_left = steps[self.step] - at;
            }
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    /// Runs and records (cycle, clocks) for every non-empty clock.
    fn trace(fc: &mut FrameCounter, until: u64) -> Vec<(u64, Clocks)> {
        let mut t = 0u64;
        let mut out = Vec::new();
        loop {
            let dt = fc.left();
            if t + dt as u64 > until {
                break;
            }
            t += dt as u64;
            let c = fc.advance(dt);
            if c.quarter || c.half {
                out.push((t, c));
            }
        }
        out
    }

    const Q: Clocks = Clocks { quarter: true, half: false };
    const QH: Clocks = Clocks { quarter: true, half: true };

    #[test]
    fn four_step_sequence() {
        let mut fc = FrameCounter::default();
        let tr = trace(&mut fc, 2 * 29830);
        assert_eq!(
            &tr[..5],
            &[(7457, Q), (14913, QH), (22371, Q), (29829, QH), (29830 + 7457, Q)]
        );
        assert!(fc.irq, "4-step mode raises the frame IRQ");
        // 240 Hz quarter / 120 Hz half on average: 4 quarters per 29830 cycles.
        let quarters = trace(&mut FrameCounter::default(), 29830 * 100).len();
        assert_eq!(quarters, 400);
    }

    #[test]
    fn five_step_sequence_and_immediate_clock() {
        let mut fc = FrameCounter::default();
        fc.write(0x80, 0); // even cycle -> 3-cycle delay
        let tr = trace(&mut fc, 3 + 2 * 37282);
        assert_eq!(tr[0], (3, QH), "5-step write clocks immediately");
        let rel: Vec<(u64, Clocks)> = tr[1..6].iter().map(|&(t, c)| (t - 3, c)).collect();
        assert_eq!(
            rel,
            [(7457, Q), (14913, QH), (22371, Q), (37281, QH), (37282 + 7457, Q)]
        );
        assert!(!fc.irq, "5-step mode never raises the IRQ");
    }

    #[test]
    fn write_delay_parity() {
        let mut fc = FrameCounter::default();
        fc.write(0x80, 1);
        assert_eq!(fc.left(), 4);
    }
}
