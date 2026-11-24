//! Transport + frame-tick scheduling around `Synth` and `Driver`.
//!
//! The song position is a continuous function of emulated CPU time:
//! `pos(cycle) = pos0 + (cycle - cycle0) * ticks_per_cycle`, wrapped at the
//! loop end. So it is sample accurate, and the engine can publish it for any
//! buffer. The driver samples it on frame ticks (every 29780.5 CPU cycles),
//! which is where note events are quantised.
//!
//! BPM -> ticks: 960 ticks per quarter note, so ticks per second =
//! BPM / 60 x 960, and per frame tick = that / 60.0988. At 150 BPM a frame
//! tick is 39.93 ticks, a 1/16 note (240 ticks) is 6.01 frames.

use crate::blep::MAX_FRAMES;
use crate::driver::{Driver, Song, Writes};
use crate::samples;
use crate::synth::Synth;
use crate::tables::{CPU_CYCLES_PER_FRAME_X2, CPU_HZ_NTSC};

pub const PPQ: f64 = 960.0;

pub struct Player {
    pub synth: Synth,
    pub driver: Driver,
    writes: Writes,
    /// Next frame tick, in half CPU cycles (a frame is 29780.5 cycles).
    next_tick_x2: u64,
    playing: bool,
    bpm: f64,
    loop_end: f64,
    /// Position (ticks) at `cycle0`.
    pos0: f64,
    cycle0: u64,
    ticks_per_cycle: f64,
    /// Position at the previous frame tick (`None` after start or seek).
    prev_tick_pos: Option<f64>,
}

impl Player {
    pub fn new(sample_rate: f64) -> Self {
        let mut synth = Synth::new(sample_rate);
        let refs = samples::build(&mut synth.rom);
        let mut p = Self {
            synth,
            driver: Driver::new(refs),
            writes: Writes::default(),
            next_tick_x2: 0,
            playing: false,
            bpm: 150.0,
            loop_end: 32.0 * 4.0 * PPQ,
            pos0: 0.0,
            cycle0: 0,
            ticks_per_cycle: 0.0,
            prev_tick_pos: None,
        };
        p.set_bpm(150.0);
        p
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn loop_end(&self) -> f64 {
        self.loop_end
    }

    fn wrap(&self, t: f64) -> f64 {
        if self.loop_end > 0.0 && t >= self.loop_end {
            t % self.loop_end
        } else {
            t
        }
    }

    pub fn position_at(&self, cycle: u64) -> f64 {
        if self.playing {
            self.wrap(self.pos0 + (cycle - self.cycle0) as f64 * self.ticks_per_cycle)
        } else {
            self.pos0
        }
    }

    /// Song position at the end of what has been rendered.
    pub fn position(&self) -> f64 {
        self.position_at(self.synth.cycle)
    }

    /// Song ticks per output sample at the current tempo.
    pub fn ticks_per_sample(&self) -> f64 {
        self.bpm / 60.0 * PPQ / self.synth.sample_rate()
    }

    fn rebase(&mut self) {
        self.pos0 = self.position();
        self.cycle0 = self.synth.cycle;
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        self.rebase();
        self.bpm = bpm;
        self.ticks_per_cycle = bpm / 60.0 * PPQ / CPU_HZ_NTSC;
    }

    pub fn set_loop_end(&mut self, ticks: f64) {
        self.rebase();
        self.loop_end = ticks.max(PPQ);
        self.pos0 = self.wrap(self.pos0);
    }

    pub fn play(&mut self) {
        if !self.playing {
            self.rebase();
            self.playing = true;
            self.prev_tick_pos = None;
        }
    }

    /// Stops in place; notes are released at the next frame tick.
    pub fn stop(&mut self) {
        self.rebase();
        self.playing = false;
    }

    pub fn seek(&mut self, ticks: f64) {
        self.pos0 = self.wrap(ticks.max(0.0));
        self.cycle0 = self.synth.cycle;
        self.prev_tick_pos = None;
    }

    pub fn set_vrc6(&mut self, on: bool) {
        self.synth.set_vrc6(on);
        self.driver.vrc6 = on;
    }

    fn frame_tick(&mut self, song: &Song) {
        let pos = self.position();
        let prev = match self.prev_tick_pos {
            // Wrapped at the loop end: everything from 0 counts as new.
            Some(p) if pos < p => Some(-1.0),
            p => p,
        };
        self.writes.clear();
        self.driver.tick(song, prev, pos, self.playing, &mut self.writes);
        for &(a, v) in self.writes.as_slice() {
            self.synth.write(a, v);
        }
        self.prev_tick_pos = if self.playing { Some(pos) } else { None };
    }

    /// Renders mono samples, running frame ticks at their exact cycles.
    pub fn render(&mut self, song: &Song, out: &mut [f32]) {
        for chunk in out.chunks_mut(MAX_FRAMES) {
            let end = self.synth.cycle_for_samples(chunk.len());
            while self.next_tick_x2 / 2 < end {
                self.synth.run_until(self.next_tick_x2 / 2);
                self.frame_tick(song);
                self.next_tick_x2 += CPU_CYCLES_PER_FRAME_X2;
            }
            self.synth.render(chunk);
        }
    }

    /// CPU cycle of the next frame tick (for tests and latency accounting).
    pub fn next_tick_cycle(&self) -> u64 {
        self.next_tick_x2 / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{Note, LANES};
    use std::boxed::Box;
    use std::vec;

    #[test]
    fn frame_ticks_at_60_0988_hz() {
        let mut p = Box::new(Player::new(48_000.0));
        let mut buf = vec![0f32; 480];
        // 10 s of audio.
        for _ in 0..1000 {
            p.render(&Song::EMPTY, &mut buf);
        }
        // Ticks at 0, 29780.5, ... up to the rendered cycle.
        let secs = p.synth.cycle as f64 / CPU_HZ_NTSC;
        assert!((secs - 10.0).abs() < 1e-3);
        let expect = (p.synth.cycle as f64 / 29780.5).ceil() as u64;
        assert_eq!(p.driver.ticks, expect);
        assert!((p.driver.ticks as f64 / secs - 60.0988).abs() < 0.2);
    }

    #[test]
    fn position_is_sample_accurate_and_wraps() {
        let mut p = Box::new(Player::new(48_000.0));
        p.set_bpm(120.0);
        p.play();
        let mut buf = vec![0f32; 4800];
        p.render(&Song::EMPTY, &mut buf); // 0.1 s
        // 120 BPM = 1920 ticks/s -> 192 ticks.
        assert!((p.position() - 192.0).abs() < 0.05, "{}", p.position());
        p.set_loop_end(960.0);
        for _ in 0..5 {
            p.render(&Song::EMPTY, &mut buf);
        }
        // 0.6 s -> 1152 ticks -> wraps to 192.
        assert!((p.position() - 192.0).abs() < 0.05, "{}", p.position());
    }

    #[test]
    fn note_starts_quantised_to_frame_ticks() {
        let mut p = Box::new(Player::new(48_000.0));
        p.set_bpm(150.0);
        // A pulse note at 1 beat (960 ticks = 0.4 s at 150 BPM).
        let notes = [Note { start: 960, end: 1920, pitch: 69 }];
        let mut lanes: [&[Note]; LANES] = [&[]; LANES];
        lanes[0] = &notes;
        let song = Song { lanes };
        p.play();
        let mut buf = vec![0f32; 48];
        let mut on_at = None;
        for i in 0..2000 {
            p.render(&song, &mut buf);
            if on_at.is_none() && p.driver.note_ons[0] == 1 {
                on_at = Some(i);
            }
        }
        // The note-on happens at the first frame tick at or after 0.4 s.
        let first_tick = (0.4 * CPU_HZ_NTSC / 29780.5).ceil() * 29780.5 / CPU_HZ_NTSC;
        let at = on_at.unwrap() as f64 * 48.0 / 48_000.0;
        assert!(at <= first_tick + 0.001 && at >= first_tick - 0.002, "{at} vs {first_tick}");
    }
}
