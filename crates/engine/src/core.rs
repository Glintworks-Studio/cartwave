//! The audio-thread side: owns the chip `Player` and the current song
//! snapshot, applies commands, renders, and publishes state.
//!
//! Rules (enforced with `NoAllocGuard` while `process` runs): no locks, no
//! allocation or freeing, no I/O or logging. Old song snapshots go back to
//! the UI thread through the garbage queue. Time stamps use `Instant::now`
//! (QueryPerformanceCounter on Windows, a user-mode read; cpal reads it on
//! this thread too).

use crate::alloc_check::NoAllocGuard;
use crate::command::Command;
use crate::shared::{PositionSnapshot, Shared};
use crate::song::SongSnapshot;
use chip::blep::DELAY as BLEP_DELAY;
use chip::driver::LANES;
use chip::player::Player;
use chip::tables::CPU_HZ_NTSC;
use rtrb::{Consumer, Producer};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

/// Largest chunk rendered at once; bigger callbacks are split.
const SCRATCH_FRAMES: usize = 4096;
/// Callbacks ignored by the underrun detectors after a stream (re)start: the
/// first ones legitimately see an empty device buffer.
const WARMUP_CALLBACKS: u64 = 3;
/// Length of the A/V sync test click.
const CLICK_FRAMES: u32 = 48;

pub struct Core {
    player: Box<Player>,
    song: Box<SongSnapshot>,
    cmds: Consumer<Command>,
    garbage: Producer<Box<SongSnapshot>>,
    shared: Arc<Shared>,
    epoch: Instant,
    scratch: Box<[f32; SCRATCH_FRAMES]>,
    sample_rate: u32,
    gain: f32,
    // Underrun detection.
    last_cb_ns: Option<u64>,
    /// Last callback of any stream (not reset on a rebuild), so the largest
    /// gap includes the silence while a stream is rebuilt.
    prev_any_cb_ns: Option<u64>,
    stream_callbacks: u64,
    /// Largest callback seen (the device buffer; the first callback gets it all).
    capacity: u32,
    /// Smallest callback seen (the device period).
    period_frames: u32,
    // Test hooks.
    stall_us: u32,
    pilot_hz: f32,
    pilot_phase: f64,
    click_at: f64,
    click_left: u32,
    commands_seen: u64,
    /// Serial numbers for audition notes (each one retriggers).
    preview_serial: u32,
}

impl Core {
    pub fn new(
        sample_rate: u32,
        cmds: Consumer<Command>,
        garbage: Producer<Box<SongSnapshot>>,
        shared: Arc<Shared>,
        epoch: Instant,
    ) -> Box<Self> {
        Box::new(Self {
            player: Box::new(Player::new(sample_rate as f64)),
            song: Box::default(),
            cmds,
            garbage,
            shared,
            epoch,
            scratch: Box::new([0.0; SCRATCH_FRAMES]),
            sample_rate,
            gain: 1.0,
            last_cb_ns: None,
            prev_any_cb_ns: None,
            stream_callbacks: 0,
            capacity: 0,
            period_frames: 0,
            stall_us: 0,
            pilot_hz: 0.0,
            pilot_phase: 0.0,
            click_at: -1.0,
            click_left: 0,
            commands_seen: 0,
            preview_serial: 0,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Called by the host after a stream (re)build, before the first callback.
    pub fn stream_started(&mut self) {
        self.last_cb_ns = None;
        self.stream_callbacks = 0;
        self.capacity = 0;
        self.period_frames = 0;
    }

    fn now_ns(&self) -> u64 {
        self.epoch.elapsed().as_nanos() as u64
    }

    fn drain_commands(&mut self) {
        loop {
            // A new song needs a free garbage slot for the old one; otherwise
            // leave it queued until the UI has collected garbage.
            if let Ok(Command::Song(_)) = self.cmds.peek() {
                if self.garbage.slots() == 0 {
                    break;
                }
            }
            let Ok(cmd) = self.cmds.pop() else { break };
            self.commands_seen += 1;
            let p = &mut self.player;
            match cmd {
                Command::Song(mut s) => {
                    let now = self.epoch.elapsed().as_nanos() as u64;
                    if s.sent_ns > 0 {
                        self.shared
                            .send_to_engine_max_ns
                            .fetch_max(now.saturating_sub(s.sent_ns), Ordering::Relaxed);
                    }
                    std::mem::swap(&mut self.song, &mut s);
                    // Cannot fail: checked above.
                    let _ = self.garbage.push(s);
                    // The driver reads the song on every frame tick: the edit
                    // is heard at the next one.
                    let wait = p.next_tick_cycle().saturating_sub(p.synth.cycle);
                    let wait_ns = (wait as f64 / CPU_HZ_NTSC * 1e9) as u64;
                    self.shared.edit_to_tick_max_ns.fetch_max(wait_ns, Ordering::Relaxed);
                    self.shared.edits_applied.fetch_add(1, Ordering::Relaxed);
                }
                Command::Play => p.play(),
                Command::Stop => p.stop(),
                Command::Seek(t) => p.seek(t),
                Command::Bpm(b) => p.set_bpm(b),
                Command::LoopEnd(t) => p.set_loop_end(t),
                Command::LoopStart(t) => p.set_loop_start(t),
                Command::Preview { lane, note } => {
                    if let Some(slot) = p.driver.preview.get_mut(lane as usize) {
                        self.preview_serial = self.preview_serial.wrapping_add(1);
                        let serial = self.preview_serial;
                        *slot = note.map(|(pitch, volume)| chip::driver::Note {
                            start: serial,
                            end: u32::MAX,
                            pitch,
                            volume,
                        });
                    }
                }
                Command::Mute(m) => p.driver.mute = m,
                Command::Solo(m) => p.driver.solo = m,
                Command::Param { lane, id, value } => {
                    if (lane as usize) < LANES {
                        p.driver.params.set(lane as usize, id, value as i32)
                    }
                }
                Command::Vrc6(on) => p.set_vrc6(on),
                Command::Filter(m) => p.synth.set_filter(m),
                Command::Gain(g) => self.gain = g,
                Command::TestStall(us) => self.stall_us = us,
                Command::TestPilot(hz) => self.pilot_hz = hz,
                Command::TestClickAt(t) => self.click_at = t,
            }
        }
    }

    /// Fills an interleaved output buffer. `ahead_ns` is cpal's
    /// playback - callback time for this buffer.
    pub fn process(&mut self, out: &mut [f32], channels: usize, ahead_ns: u64) {
        let _no_alloc = NoAllocGuard::new();
        let t0 = self.now_ns();
        let channels = channels.max(1);
        let frames = (out.len() / channels) as u32;
        let sr = self.sample_rate as u64;
        let period_ns = frames as u64 * 1_000_000_000 / sr;
        // A second handle (an atomic increment, no allocation) so `self` stays free.
        let shared = self.shared.clone();
        let sh = &*shared;
        sh.audio_tid
            .store(crate::cpu::current_thread_id(), Ordering::Relaxed);

        // ---- underrun detectors
        self.stream_callbacks += 1;
        let warm = self.stream_callbacks > WARMUP_CALLBACKS;
        // (1) Starved: WASAPI asks for more than one device period only after
        // the device ran dry (validated against loopback capture: every stall
        // that produced a dropout, and none that didn't; ENGINE.md). The margin
        // tolerates drivers that alternate e.g. 441/442 frames.
        if warm
            && self.period_frames > 0
            && frames >= self.period_frames + (self.period_frames / 16).max(16)
        {
            sh.xrun_starved.fetch_add(1, Ordering::Relaxed);
        }
        if self.period_frames == 0 || frames < self.period_frames {
            self.period_frames = frames;
        }
        self.capacity = self.capacity.max(frames);
        // (2) Late: more time passed since the last callback than the whole
        // device buffer holds. Coarser on WASAPI (it missed 22-23 ms stalls on
        // a 23.5 ms buffer); kept as an independent second signal.
        if let Some(prev) = self.prev_any_cb_ns {
            sh.max_gap_ns.fetch_max(t0 - prev, Ordering::Relaxed);
        }
        self.prev_any_cb_ns = Some(t0);
        if let Some(last) = self.last_cb_ns {
            let gap = t0 - last;
            let capacity_ns = self.capacity as u64 * 1_000_000_000 / sr;
            // CoreAudio calls back with exactly one period every time, so the
            // largest callback is the period and ordinary scheduling jitter
            // (measured up to 2.3% at 64 frames) would cross it. When no
            // callback has been bigger than the period, allow 1/8 of it (at
            // least 0.25 ms). WASAPI's first callback is the whole device
            // buffer, so its threshold is unchanged.
            let jitter_ns = if self.capacity == self.period_frames {
                (capacity_ns / 8).max(250_000)
            } else {
                0
            };
            if warm && gap > capacity_ns + jitter_ns {
                sh.xrun_late.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.last_cb_ns = Some(t0);
        sh.cb_frames_last.store(frames, Ordering::Relaxed);
        sh.cb_frames_max.fetch_max(frames, Ordering::Relaxed);
        let min = sh.cb_frames_min.load(Ordering::Relaxed);
        if min == 0 || frames < min {
            sh.cb_frames_min.store(frames, Ordering::Relaxed);
        }
        sh.ahead_ns.store(ahead_ns, Ordering::Relaxed);

        self.drain_commands();

        // ---- render
        let pos0 = self.player.position();
        let tpf = self.player.ticks_per_sample();
        let playing = self.player.playing();
        let song = self.song.view();
        let mut done = 0usize;
        while done < frames as usize {
            let n = (frames as usize - done).min(SCRATCH_FRAMES);
            let buf = &mut self.scratch[..n];
            self.player.render(&song, buf);
            // Test: click when the song position crosses `click_at`.
            if self.click_at >= 0.0 && playing && self.click_left == 0 {
                let p = pos0 + done as f64 * tpf;
                let k = ((self.click_at - p) / tpf).ceil();
                if k >= 0.0 && (k as usize) < n {
                    self.click_left = CLICK_FRAMES;
                    for v in &mut buf[k as usize..] {
                        if self.click_left == 0 {
                            break;
                        }
                        *v = 0.9;
                        self.click_left -= 1;
                    }
                }
            } else if self.click_left > 0 {
                for v in buf.iter_mut() {
                    if self.click_left == 0 {
                        break;
                    }
                    *v = 0.9;
                    self.click_left -= 1;
                }
            }
            if self.pilot_hz > 0.0 {
                let inc = self.pilot_hz as f64 / sr as f64;
                for v in buf.iter_mut() {
                    self.pilot_phase = (self.pilot_phase + inc).fract();
                    *v += 0.01 * (self.pilot_phase * std::f64::consts::TAU).sin() as f32;
                }
            }
            let dst = &mut out[done * channels..(done + n) * channels];
            for (frame, &v) in dst.chunks_exact_mut(channels).zip(buf.iter()) {
                frame.fill(v * self.gain);
            }
            done += n;
        }

        // ---- publish
        let blep_ns = BLEP_DELAY as u64 * 1_000_000_000 / sr;
        sh.publish_position(&PositionSnapshot {
            ticks: pos0,
            ticks_per_frame: tpf,
            loop_start: self.player.loop_start(),
            loop_end: self.player.loop_end(),
            playing,
            callback_ns: t0,
            latency_ns: ahead_ns + blep_ns,
            sample_rate: self.sample_rate,
        });
        let levels = self.player.synth.take_peaks();
        sh.merge_levels(&levels);
        for (a, &v) in sh.note_ons.iter().zip(self.player.driver.note_ons.iter()) {
            a.store(v, Ordering::Relaxed);
        }
        sh.sounding.store(self.player.driver.sounding, Ordering::Relaxed);
        sh.driver_ticks.store(self.player.driver.ticks, Ordering::Relaxed);
        sh.frames.fetch_add(frames as u64, Ordering::Relaxed);
        // After the position: a reader that sees this count sees that position.
        sh.commands.store(self.commands_seen, Ordering::Release);

        // ---- test stall (busy-wait, like a long computation would)
        if self.stall_us > 0 {
            let until = t0 + self.stall_us as u64 * 1000;
            self.stall_us = 0;
            while self.now_ns() < until {
                std::hint::spin_loop();
            }
        }

        let dur = self.now_ns() - t0;
        sh.record_callback(dur, period_ns);
        if dur > period_ns {
            sh.xrun_overrun.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::ClipInput;

    fn core() -> (Box<Core>, Producer<Command>, Consumer<Box<SongSnapshot>>, Arc<Shared>) {
        let (tx, rx) = rtrb::RingBuffer::new(64);
        let (gtx, grx) = rtrb::RingBuffer::new(4);
        let shared = Arc::new(Shared::default());
        let c = Core::new(48_000, rx, gtx, shared.clone(), Instant::now());
        (c, tx, grx, shared)
    }

    #[test]
    fn plays_and_publishes() {
        let (mut c, mut tx, mut grx, shared) = core();
        let notes = [(0, 960, 69u8, None)];
        let s = SongSnapshot::build([ClipInput { lane: 0, start: 0, len: 960, notes: &notes }], 1);
        tx.push(Command::Song(Box::new(s))).ok().unwrap();
        tx.push(Command::Play).ok().unwrap();
        let mut out = vec![0f32; 512 * 2];
        for _ in 0..10 {
            c.process(&mut out, 2, 30_000_000);
        }
        let p = shared.position().unwrap();
        assert!(p.playing && p.ticks > 0.0);
        assert!(out.iter().any(|&v| v != 0.0));
        assert_eq!(out[0], out[1], "mono duplicated to both channels");
        assert_eq!(shared.note_ons[0].load(Ordering::Relaxed), 1);
        assert_eq!(shared.edits_applied.load(Ordering::Relaxed), 1);
        // The old (empty) snapshot came back for freeing on this thread.
        assert!(grx.pop().is_ok());
        let wait = shared.edit_to_tick_max_ns.load(Ordering::Relaxed);
        assert!(wait <= 16_640_000, "edit heard within one frame tick: {wait}");
    }

    #[test]
    fn song_waits_for_garbage_space() {
        let (mut c, mut tx, mut grx, shared) = core();
        for v in 0..6 {
            tx.push(Command::Song(Box::default())).ok().unwrap();
            let _ = v;
        }
        let mut out = vec![0f32; 256 * 2];
        c.process(&mut out, 2, 0);
        assert_eq!(shared.edits_applied.load(Ordering::Relaxed), 4, "garbage queue holds 4");
        while grx.pop().is_ok() {}
        c.process(&mut out, 2, 0);
        assert_eq!(shared.edits_applied.load(Ordering::Relaxed), 6);
    }

    #[test]
    fn detects_injected_stall_as_late_callback() {
        let (mut c, mut tx, _grx, shared) = core();
        let mut out = vec![0f32; 512 * 2];
        // Simulate a real-time cadence: process, then sleep the period.
        for i in 0..12 {
            if i == 8 {
                tx.push(Command::TestStall(40_000)).ok().unwrap();
            }
            c.process(&mut out, 2, 0);
            std::thread::sleep(std::time::Duration::from_micros(10_000));
        }
        assert!(shared.xrun_late.load(Ordering::Relaxed) >= 1);
        assert!(shared.xrun_overrun.load(Ordering::Relaxed) >= 1);
    }
}
