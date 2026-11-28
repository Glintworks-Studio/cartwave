//! Instruments: per-frame macro sequences, the way chip music drivers shape
//! sound on hardware that has no ADSR, LFO or filter.
//!
//! Every frame tick (60.1 Hz) the driver steps each sounding lane's macros and
//! writes the resulting registers:
//!
//! - **volume** (0..=15): multiplied with the note's (or lane's) volume.
//!   Triangle: 0 silences, anything else plays. Saw: scales the accumulator rate.
//! - **arpeggio** (semitones, absolute offset from the note). On noise, one
//!   step = one noise period index (positive = higher).
//! - **pitch** (1/16 semitone per frame, *accumulating*): a value of -16 held
//!   in a loop falls one semitone per frame (a laser).
//! - **duty** (in 1/16ths of the cycle: 2 = 12.5%, 4 = 25%, 8 = 50%, 12 = 75%).
//!   The 2A03 pulse takes the nearest of its four; the VRC6 pulse uses 1..=8
//!   directly (12/16 sounds like 4/16). On noise: 0 = long mode, else short.
//!
//! A sequence can loop (`loop_at`) and has an optional release point: while
//! the note is held it loops (or holds) within `..=release_at`, and when the
//! note ends it continues from `release_at + 1`, so instruments get natural
//! tails. Without a release point a note ends instantly, like hardware.
//!
//! A **kit** maps pitch zones to other instruments of the song's bank, so one
//! noise lane can play a kick, a snare and hats with their own envelopes.
//!
//! `no_std`, `Copy`, fixed size: the engine stores instruments inline.

/// Steps per sequence (about one second at 60 Hz).
pub const SEQ_LEN: usize = 64;
/// "No index" marker for `loop_at` / `release_at`.
pub const NONE: u8 = 0xFF;
/// Pitch-zone slots in a kit.
pub const KIT_ZONES: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Seq {
    pub len: u8,
    pub loop_at: u8,
    pub release_at: u8,
    pub data: [i8; SEQ_LEN],
}

impl Default for Seq {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Seq {
    pub const EMPTY: Seq = Seq { len: 0, loop_at: NONE, release_at: NONE, data: [0; SEQ_LEN] };

    /// A sequence from values; `loop_at` and `release_at` are step indices
    /// (`NONE` for none). Values past `SEQ_LEN` are dropped.
    pub const fn new(values: &[i8], loop_at: u8, release_at: u8) -> Seq {
        let mut data = [0i8; SEQ_LEN];
        let n = if values.len() < SEQ_LEN { values.len() } else { SEQ_LEN };
        let mut i = 0;
        while i < n {
            data[i] = values[i];
            i += 1;
        }
        let loop_at = if (loop_at as usize) < n { loop_at } else { NONE };
        let release_at = if (release_at as usize) < n { release_at } else { NONE };
        Seq { len: n as u8, loop_at, release_at, data }
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn values(&self) -> &[i8] {
        &self.data[..self.len as usize]
    }

    pub fn has_release(&self) -> bool {
        self.release_at != NONE
    }

    /// The value at `pos`, or `default` for an empty sequence.
    pub fn at(&self, pos: u8, default: i8) -> i8 {
        if self.len == 0 {
            default
        } else {
            self.data[(pos as usize).min(self.len as usize - 1)]
        }
    }

    /// Advances `pos` by one frame.
    pub fn step(&self, pos: u8, released: bool) -> u8 {
        if self.len == 0 {
            return 0;
        }
        let last = self.len - 1;
        if !released && self.release_at != NONE && pos >= self.release_at {
            // Held: loop within the sustain part, or sit on the release point.
            return if self.loop_at != NONE && self.loop_at <= self.release_at {
                self.loop_at
            } else {
                self.release_at
            };
        }
        if pos >= last {
            let can_loop = self.loop_at != NONE
                && (self.release_at == NONE || !released || self.loop_at > self.release_at);
            return if can_loop { self.loop_at } else { last };
        }
        pos + 1
    }

    /// Where the sequence continues when the note is released.
    pub fn release_pos(&self, pos: u8) -> u8 {
        if self.release_at != NONE && self.release_at + 1 < self.len {
            self.release_at + 1
        } else {
            pos
        }
    }

    /// The release tail has played out (or there is none to play).
    pub fn finished(&self, pos: u8, released: bool) -> bool {
        if !released {
            return false;
        }
        if self.release_at == NONE || self.len == 0 {
            return true;
        }
        // A loop after the release point never finishes on its own.
        pos >= self.len - 1 && !(self.loop_at != NONE && self.loop_at > self.release_at)
    }
}

/// Delayed vibrato (a sine LFO on pitch).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct Vibrato {
    /// Peak deviation in cents (0 = off).
    pub depth: u8,
    /// Frames per cycle (6 = 10 Hz).
    pub period: u8,
    /// Frames before it starts; it then fades in over the same time.
    pub delay: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KitZone {
    /// Highest MIDI pitch of this zone (inclusive); zones are checked in order.
    pub top: u8,
    /// Index into the song's instrument bank.
    pub inst: u8,
    /// Pitch actually played (`NONE` = the note's own).
    pub fixed_pitch: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct Instrument {
    pub volume: Seq,
    pub arp: Seq,
    pub pitch: Seq,
    pub duty: Seq,
    pub vibrato: Vibrato,
    /// Frames per arpeggio step (0 and 1: every frame, the classic fast
    /// arpeggio). The driver holds each arpeggio step this many frames, as
    /// game sound drivers' sequence speeds did: a driver feature, so it is
    /// allowed on locked tracks.
    pub arp_speed: u8,
    /// Number of used `kit` zones; > 0 makes this a kit.
    pub kit_len: u8,
    pub kit: [Option<KitZone>; KIT_ZONES],
}

impl Instrument {
    /// No macros: the channel's own controls only (the pre-instrument sound).
    pub const PLAIN: Instrument = Instrument {
        volume: Seq::EMPTY,
        arp: Seq::EMPTY,
        pitch: Seq::EMPTY,
        duty: Seq::EMPTY,
        vibrato: Vibrato { depth: 0, period: 0, delay: 0 },
        arp_speed: 1,
        kit_len: 0,
        kit: [None; KIT_ZONES],
    };

    pub fn is_kit(&self) -> bool {
        self.kit_len > 0
    }

    /// The zone for `pitch`, if this is a kit.
    pub fn zone(&self, pitch: u8) -> Option<KitZone> {
        self.kit[..self.kit_len as usize]
            .iter()
            .flatten()
            .find(|z| pitch <= z.top)
            .copied()
    }

    pub fn has_release(&self) -> bool {
        self.volume.has_release()
    }
}

/// Per-lane playback position in the macros.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MacroState {
    pub pos: [u8; 4],
    pub released: bool,
    /// Accumulated pitch macro, 1/16 semitones.
    pub pitch_acc: i32,
    /// Frames since note-on.
    pub frames: u32,
    /// Frames the arpeggio has held its current step (`Instrument::arp_speed`).
    pub arp_hold: u8,
}

/// Most frames per arpeggio step.
pub const ARP_SPEED_MAX: u8 = 8;

/// The values one frame of an instrument produces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// 0..=15 (15 when there is no volume macro).
    pub volume: u8,
    pub arp: i32,
    /// Fine offset in semitones (pitch macro + vibrato).
    pub fine: f64,
    /// Duty in 1/16ths, or `None` to use the channel control.
    pub duty: Option<u8>,
}

impl MacroState {
    pub fn start() -> Self {
        Self::default()
    }

    /// The current frame's values (call `advance` once per later frame first).
    pub fn frame(&self, inst: &Instrument) -> Frame {
        let vol = inst.volume.at(self.pos[0], 15).clamp(0, 15) as u8;
        let arp = inst.arp.at(self.pos[1], 0) as i32;
        let duty = if inst.duty.is_empty() {
            None
        } else {
            Some(inst.duty.at(self.pos[3], 8).clamp(0, 16) as u8)
        };
        let mut fine = self.pitch_acc as f64 / 16.0;
        let v = inst.vibrato;
        if v.depth > 0 && v.period > 0 && self.frames >= v.delay as u32 {
            let t = self.frames - v.delay as u32;
            let fade = if v.delay == 0 { 1.0 } else { (t as f64 / v.delay as f64).min(1.0) };
            let phase = (t % v.period as u32) as f64 / v.period as f64;
            fine += fade * v.depth as f64 / 100.0 * crate::math::sin(phase * 2.0 * core::f64::consts::PI);
        }
        Frame { volume: vol, arp, fine, duty }
    }

    /// The pitch macro's value is added when its step is entered.
    fn enter_pitch(&mut self, inst: &Instrument) {
        if !inst.pitch.is_empty() {
            self.pitch_acc = (self.pitch_acc + inst.pitch.at(self.pos[2], 0) as i32).clamp(-16 * 96, 16 * 96);
        }
    }

    /// State at note-on (the first frame's pitch step applied).
    pub fn note_on(inst: &Instrument) -> Self {
        let mut s = Self::start();
        s.enter_pitch(inst);
        s
    }

    pub fn advance(&mut self, inst: &Instrument) {
        let seqs = [&inst.volume, &inst.arp, &inst.pitch, &inst.duty];
        // The arpeggio moves on every `arp_speed`-th frame (1: every frame).
        let speed = inst.arp_speed.clamp(1, ARP_SPEED_MAX);
        let arp_moves = if speed <= 1 {
            true
        } else {
            self.arp_hold += 1;
            if self.arp_hold >= speed {
                self.arp_hold = 0;
                true
            } else {
                false
            }
        };
        for (i, s) in seqs.iter().enumerate() {
            if i == 1 && !arp_moves {
                continue;
            }
            self.pos[i] = s.step(self.pos[i], self.released);
        }
        self.frames = self.frames.saturating_add(1);
        self.enter_pitch(inst);
    }

    pub fn release(&mut self, inst: &Instrument) {
        if self.released {
            return;
        }
        self.released = true;
        let seqs = [&inst.volume, &inst.arp, &inst.pitch, &inst.duty];
        for (i, s) in seqs.iter().enumerate() {
            self.pos[i] = s.release_pos(self.pos[i]);
        }
    }

    /// The release tail is over: the note can be cut.
    pub fn done(&self, inst: &Instrument) -> bool {
        inst.volume.finished(self.pos[0], self.released)
    }
}

/// Volume after the macro: `ceil(base * macro / 15)`, at least 1 when both
/// are non-zero (so quiet notes don't vanish), as chip drivers do.
pub fn scale_volume(base: u8, mac: u8) -> u8 {
    if base == 0 || mac == 0 {
        return 0;
    }
    (((base as u32) * (mac as u32)).div_ceil(15)).clamp(1, 15) as u8
}

/// 2A03 duty index (0..=3) for a duty in 1/16ths.
pub fn duty_2a03(sixteenths: u8) -> u8 {
    match sixteenths {
        0..=3 => 0,
        4..=6 => 1,
        7..=10 => 2,
        _ => 3,
    }
}

/// VRC6 duty field (0..=7, (d+1)/16) for a duty in 1/16ths.
pub fn duty_vrc6(sixteenths: u8) -> u8 {
    let s = if sixteenths > 8 { 16 - sixteenths.min(15) } else { sixteenths };
    s.clamp(1, 8) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sustain_loop_then_release_tail() {
        // Attack 0..2, sustain loop 2..=3, release 4..6.
        let s = Seq::new(&[5, 10, 12, 11, 8, 4, 0], 2, 3);
        let mut p = 0;
        let mut seen = std::vec::Vec::new();
        for _ in 0..8 {
            seen.push(s.at(p, 0));
            p = s.step(p, false);
        }
        assert_eq!(seen, [5, 10, 12, 11, 12, 11, 12, 11]);
        p = s.release_pos(p);
        let mut tail = std::vec::Vec::new();
        while !s.finished(p, true) {
            tail.push(s.at(p, 0));
            p = s.step(p, true);
        }
        assert_eq!(tail, [8, 4]);
        assert_eq!(s.at(p, 0), 0);
    }

    #[test]
    fn hold_last_and_loop() {
        let d = Seq::new(&[15, 10, 6], NONE, NONE);
        assert_eq!(d.step(2, false), 2);
        let l = Seq::new(&[0, 4, 7], 0, NONE);
        assert_eq!(l.step(2, false), 0);
        assert!(Seq::new(&[1, 2], 5, 9).loop_at == NONE);
    }

    #[test]
    fn volume_scaling_and_duty_maps() {
        assert_eq!(scale_volume(15, 15), 15);
        assert_eq!(scale_volume(12, 1), 1);
        assert_eq!(scale_volume(0, 15), 0);
        assert_eq!(scale_volume(8, 8), 5);
        assert_eq!([2, 4, 8, 12].map(duty_2a03), [0, 1, 2, 3]);
        assert_eq!([1, 4, 8, 12].map(duty_vrc6), [0, 3, 7, 3]);
    }

    #[test]
    fn pitch_accumulates_and_vibrato_fades_in() {
        let inst = Instrument {
            pitch: Seq::new(&[-16], 0, NONE),
            vibrato: Vibrato { depth: 50, period: 4, delay: 0 },
            ..Instrument::PLAIN
        };
        let mut st = MacroState::note_on(&inst);
        assert_eq!(st.pitch_acc, -16);
        st.advance(&inst);
        st.advance(&inst);
        assert_eq!(st.pitch_acc, -48);
        // Vibrato at phase 1/4 of the cycle: +depth.
        let f = MacroState { frames: 1, pitch_acc: 0, ..Default::default() }.frame(&inst);
        assert!((f.fine - 0.5).abs() < 1e-6, "{}", f.fine);
    }

    #[test]
    fn arp_speed_holds_each_step() {
        let arp = Seq::new(&[0, 4, 7], 0, NONE);
        let seen = |speed: u8| {
            let inst = Instrument { arp, arp_speed: speed, ..Instrument::PLAIN };
            let mut st = MacroState::note_on(&inst);
            let mut v = std::vec::Vec::new();
            for _ in 0..9 {
                v.push(st.frame(&inst).arp);
                st.advance(&inst);
            }
            v
        };
        assert_eq!(seen(1), [0, 4, 7, 0, 4, 7, 0, 4, 7]);
        // 0 is treated as 1 (every frame).
        assert_eq!(seen(0), seen(1));
        assert_eq!(seen(3), [0, 0, 0, 4, 4, 4, 7, 7, 7]);
        // The other macros keep stepping every frame.
        let inst = Instrument { arp, arp_speed: 3, volume: Seq::new(&[15, 10, 5], NONE, NONE), ..Instrument::PLAIN };
        let mut st = MacroState::note_on(&inst);
        st.advance(&inst);
        assert_eq!(st.frame(&inst).volume, 10);
        assert_eq!(st.frame(&inst).arp, 0);
    }

    #[test]
    fn kit_zones() {
        let mut k = Instrument::PLAIN;
        k.kit_len = 2;
        k.kit[0] = Some(KitZone { top: 47, inst: 3, fixed_pitch: NONE });
        k.kit[1] = Some(KitZone { top: 127, inst: 4, fixed_pitch: 70 });
        assert_eq!(k.zone(40).map(|z| z.inst), Some(3));
        assert_eq!(k.zone(60).map(|z| z.inst), Some(4));
        assert!(k.is_kit());
    }
}
