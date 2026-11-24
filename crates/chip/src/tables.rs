//! Hardware constants and tables, transcribed from the NESdev wiki
//! (APU, APU Length Counter, APU Pulse, APU Triangle, APU Noise, APU DMC,
//! APU Frame Counter, CPU, Cycle reference chart). See ENGINE.md, "Sources".

/// NTSC master clock: 236.25 MHz / 11 = 21.477272... MHz (6 x colour subcarrier).
pub const MASTER_HZ_NTSC: f64 = 236_250_000.0 / 11.0;
/// NTSC CPU (and APU input) clock: master / 12 = 1.789772727... MHz.
pub const CPU_HZ_NTSC: f64 = MASTER_HZ_NTSC / 12.0;
/// Twice the CPU cycles per NTSC video frame with rendering enabled:
/// (341 x 261 + 340.5) PPU dots / 3 = 29780.5 CPU cycles.
pub const CPU_CYCLES_PER_FRAME_X2: u64 = 59_561;
/// NTSC video frame rate, Hz (derived: CPU / 29780.5 = 60.0988...).
pub const FRAME_HZ_NTSC: f64 = CPU_HZ_NTSC * 2.0 / CPU_CYCLES_PER_FRAME_X2 as f64;

/// Length counter load values, indexed by bits 7-3 of $4003/$4007/$400B/$400F.
pub const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, //
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

/// Pulse duty waveforms in output order, starting from the step a
/// $4003/$4007 write resets the sequencer to: 12.5%, 25%, 50%, 25% negated.
pub const PULSE_DUTY: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

/// Triangle 32-step sequence.
pub const TRIANGLE_SEQ: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, //
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

/// Noise timer periods in CPU cycles (NTSC; $F is 4068 on all but the earliest 2A03s).
pub const NOISE_PERIOD_NTSC: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

/// DMC output-unit timer periods in CPU cycles (NTSC).
pub const DMC_RATE_NTSC: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

/// Frame counter step times in CPU cycles after the counter is reset.
/// The wiki gives APU-cycle times 3728.5, 7456.5, 11185.5, 14914.5 (and 18640.5
/// in 5-step mode); one APU cycle = 2 CPU cycles.
pub const FRAME_STEPS_4: [u32; 4] = [7457, 14913, 22371, 29829];
/// 4-step sequence length (2 x 14915).
pub const FRAME_PERIOD_4: u32 = 29830;
pub const FRAME_STEPS_5: [u32; 5] = [7457, 14913, 22371, 29829, 37281];
/// 5-step sequence length (2 x 18641).
pub const FRAME_PERIOD_5: u32 = 37282;

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn clocks() {
        assert!((CPU_HZ_NTSC - 1_789_772.727).abs() < 0.001);
        assert!((FRAME_HZ_NTSC - 60.0988).abs() < 0.0001);
    }

    #[test]
    fn length_table_structure() {
        // Odd indices are linear: 1 -> 254, 3 -> 2, 5 -> 4 ... 31 -> 30.
        assert_eq!(LENGTH_TABLE[1], 254);
        for i in 1..16 {
            assert_eq!(LENGTH_TABLE[2 * i + 1] as usize, 2 * i);
        }
        // Even indices: note lengths on a base of 10 (MSB clear) or 12 (MSB set).
        let lo: Vec<u8> = LENGTH_TABLE[0..16].iter().step_by(2).copied().collect();
        let hi: Vec<u8> = LENGTH_TABLE[16..32].iter().step_by(2).copied().collect();
        assert_eq!(lo, [10, 20, 40, 80, 160, 60, 14, 26]);
        assert_eq!(hi, [12, 24, 48, 96, 192, 72, 16, 32]);
    }

    #[test]
    fn frame_steps_are_apu_times_doubled() {
        let apu = [3728.5f64, 7456.5, 11185.5, 14914.5];
        for (s, a) in FRAME_STEPS_4.iter().zip(apu) {
            assert_eq!(*s as f64, a * 2.0);
        }
        assert_eq!(FRAME_STEPS_5[4] as f64, 18640.5 * 2.0);
        assert_eq!(FRAME_PERIOD_4, 2 * 14915);
        assert_eq!(FRAME_PERIOD_5, 2 * 18641);
    }

    #[test]
    fn duty_shares() {
        let ones: Vec<u32> = PULSE_DUTY
            .iter()
            .map(|d| d.iter().map(|&b| b as u32).sum())
            .collect();
        assert_eq!(ones, [1, 2, 4, 6]);
    }
}
