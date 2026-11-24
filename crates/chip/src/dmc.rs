//! 2A03 delta modulation channel. NESdev "APU DMC".
//!
//! The memory reader reads from a 16 KiB sample ROM mapped at $C000-$FFFF
//! (addresses $8000-$BFFF read as 0). We only ever put our own synthesised
//! samples there. There is no CPU, so the DMA stall (1-4 cycles) is not modelled.

use crate::tables::DMC_RATE_NTSC;

pub const ROM_BASE: u16 = 0xC000;
pub const ROM_SIZE: usize = 0x4000;

#[derive(Clone, Copy, Debug)]
pub struct Dmc {
    pub irq_enable: bool,
    pub loop_flag: bool,
    pub rate_index: u8,
    /// 7-bit output level (the DAC input).
    pub level: u8,
    sample_addr: u16,
    sample_len: u16,
    // Memory reader
    address: u16,
    pub bytes_remaining: u16,
    buffer: Option<u8>,
    // Output unit
    shift: u8,
    bits_remaining: u8,
    silence: bool,
    pub timer_left: u32,
    pub irq: bool,
}

impl Default for Dmc {
    fn default() -> Self {
        Self {
            irq_enable: false,
            loop_flag: false,
            rate_index: 0,
            level: 0,
            sample_addr: ROM_BASE,
            sample_len: 1,
            address: ROM_BASE,
            bytes_remaining: 0,
            buffer: None,
            shift: 0,
            bits_remaining: 8,
            silence: true,
            timer_left: DMC_RATE_NTSC[0] as u32,
            irq: false,
        }
    }
}

impl Dmc {
    /// `reg` is 0..=3 for $4010-$4013.
    pub fn write(&mut self, reg: u16, v: u8) {
        match reg {
            0 => {
                self.irq_enable = v & 0x80 != 0;
                self.loop_flag = v & 0x40 != 0;
                self.rate_index = v & 0x0F;
                if !self.irq_enable {
                    self.irq = false;
                }
            }
            1 => self.level = v & 0x7F,
            2 => self.sample_addr = 0xC000 | ((v as u16) << 6),
            _ => self.sample_len = ((v as u16) << 4) + 1,
        }
    }

    /// $4015 bit 4. Clearing stops after the current byte; setting restarts
    /// only if no bytes remain.
    pub fn set_enabled(&mut self, on: bool, rom: &[u8; ROM_SIZE]) {
        self.irq = false;
        if !on {
            self.bytes_remaining = 0;
        } else if self.bytes_remaining == 0 {
            self.restart();
            self.fill(rom);
        }
    }

    fn restart(&mut self) {
        self.address = self.sample_addr;
        self.bytes_remaining = self.sample_len;
    }

    /// Fetch a byte if the buffer is empty and bytes remain.
    fn fill(&mut self, rom: &[u8; ROM_SIZE]) {
        if self.buffer.is_some() || self.bytes_remaining == 0 {
            return;
        }
        let a = self.address;
        self.buffer = Some(if a >= ROM_BASE {
            rom[(a - ROM_BASE) as usize]
        } else {
            0
        });
        self.address = if a == 0xFFFF { 0x8000 } else { a + 1 };
        self.bytes_remaining -= 1;
        if self.bytes_remaining == 0 {
            if self.loop_flag {
                self.restart();
            } else if self.irq_enable {
                self.irq = true;
            }
        }
    }

    /// Output unit clock. Returns whether the level changed.
    #[inline]
    pub fn clock_timer(&mut self, rom: &[u8; ROM_SIZE]) -> bool {
        self.timer_left = DMC_RATE_NTSC[self.rate_index as usize] as u32;
        let old = self.level;
        if !self.silence {
            if self.shift & 1 != 0 {
                if self.level <= 125 {
                    self.level += 2;
                }
            } else if self.level >= 2 {
                self.level -= 2;
            }
        }
        self.shift >>= 1;
        self.bits_remaining -= 1;
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;
            match self.buffer.take() {
                Some(b) => {
                    self.silence = false;
                    self.shift = b;
                    self.fill(rom);
                }
                None => self.silence = true,
            }
        }
        old != self.level
    }

    pub fn active(&self) -> bool {
        self.bytes_remaining > 0
    }

    #[inline]
    pub fn output(&self) -> u8 {
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::boxed::Box;
    use std::vec::Vec;

    fn rom_with(bytes: &[u8]) -> Box<[u8; ROM_SIZE]> {
        let mut r = Box::new([0u8; ROM_SIZE]);
        r[..bytes.len()].copy_from_slice(bytes);
        r
    }

    fn clock_n(d: &mut Dmc, rom: &[u8; ROM_SIZE], n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                d.clock_timer(rom);
                d.level
            })
            .collect()
    }

    #[test]
    fn delta_steps_of_two_lsb_first_with_clamping() {
        let rom = rom_with(&[0b0000_0101]);
        let mut d = Dmc::default();
        d.write(1, 64);
        d.write(3, 0); // length 1 byte
        d.set_enabled(true, &rom);
        // The first 8 clocks drain the (empty) shift register while silent.
        let silent = clock_n(&mut d, &rom, 8);
        assert!(silent.iter().all(|&l| l == 64));
        let out = clock_n(&mut d, &rom, 8);
        // Bits LSB first: 1,0,1,0,0,0,0,0 -> +2, -2, +2, -2, -2, -2, -2, -2.
        assert_eq!(out, [66, 64, 66, 64, 62, 60, 58, 56]);

        // Clamping: at 126/127 a 1-bit doesn't increment; at 0/1 a 0-bit doesn't decrement.
        let rom = rom_with(&[0xFF, 0x00]);
        let mut d = Dmc::default();
        d.write(1, 125);
        d.write(3, 0);
        d.write(2, 0);
        d.sample_len = 2;
        d.set_enabled(true, &rom);
        clock_n(&mut d, &rom, 8);
        let up = clock_n(&mut d, &rom, 8);
        assert_eq!(up, [127, 127, 127, 127, 127, 127, 127, 127]);
        d.write(1, 1);
        let down = clock_n(&mut d, &rom, 8);
        assert_eq!(down, [1; 8]);
    }

    #[test]
    fn address_length_and_loop() {
        let rom = rom_with(&[]);
        let mut d = Dmc::default();
        d.write(2, 0x01);
        d.write(3, 0x02);
        assert_eq!(d.sample_addr, 0xC040, "addr = $C000 + A*64");
        assert_eq!(d.sample_len, 0x21, "len = L*16 + 1");
        d.write(0, 0x40 | 0x0F); // loop, rate 15
        d.set_enabled(true, &rom);
        // Never ends while looping.
        clock_n(&mut d, &rom, 8 * 0x21 * 3);
        assert!(d.active());
        d.set_enabled(false, &rom);
        assert!(!d.active());
    }

    #[test]
    fn irq_at_end_without_loop() {
        let rom = rom_with(&[]);
        let mut d = Dmc::default();
        d.write(0, 0x80);
        d.write(3, 0);
        d.set_enabled(true, &rom); // fetches the single byte immediately
        assert!(d.irq);
        assert!(!d.active());
    }

    #[test]
    fn rate_table() {
        let rom = rom_with(&[]);
        let mut d = Dmc::default();
        for (i, &p) in DMC_RATE_NTSC.iter().enumerate() {
            d.write(0, i as u8);
            d.clock_timer(&rom);
            assert_eq!(d.timer_left, p as u32);
        }
        // Rate $F: 1.789773 MHz / 54 = 33143.9 Hz (wiki).
        assert!((crate::tables::CPU_HZ_NTSC / 54.0 - 33143.9).abs() < 0.1);
    }
}
