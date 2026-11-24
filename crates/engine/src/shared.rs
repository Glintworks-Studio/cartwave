//! Engine -> UI state, published by the audio thread with atomics only.
//! The UI reads it on its own frame tick; the audio thread never calls the UI.

use chip::driver::LANES;
use std::sync::atomic::{fence, AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};

/// Callback-time histogram: buckets of 0.1% of the buffer period, 0..20%,
/// plus one overflow bucket (the exact maximum is kept separately).
pub const HIST_BUCKETS: usize = 201;
pub const HIST_STEP_PERMILLE: u64 = 1;

/// Position snapshot, written once per callback under a sequence lock.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PositionSnapshot {
    /// Song position (ticks) of the first frame of the last rendered buffer.
    pub ticks: f64,
    /// Song ticks per output frame.
    pub ticks_per_frame: f64,
    pub loop_end: f64,
    pub playing: bool,
    /// When the callback that rendered it started (ns since the engine epoch).
    pub callback_ns: u64,
    /// From the callback start until the first frame of that buffer is played
    /// (cpal's playback - callback timestamps, plus the band-limiting delay).
    pub latency_ns: u64,
    pub sample_rate: u32,
}

#[derive(Default)]
struct Seq {
    seq: AtomicU32,
    ticks: AtomicU64,
    tpf: AtomicU64,
    loop_end: AtomicU64,
    playing: AtomicBool,
    callback_ns: AtomicU64,
    latency_ns: AtomicU64,
    sample_rate: AtomicU32,
}

pub struct Hist(pub [AtomicU32; HIST_BUCKETS]);

impl Default for Hist {
    fn default() -> Self {
        Self(std::array::from_fn(|_| AtomicU32::new(0)))
    }
}

impl std::ops::Deref for Hist {
    type Target = [AtomicU32; HIST_BUCKETS];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct Shared {
    pos: Seq,
    /// Per-lane activity (f32 bits, max-merged; the UI takes and clears).
    levels: [AtomicU32; LANES],
    pub note_ons: [AtomicU32; LANES],
    pub sounding: AtomicU8,

    // ---- statistics (monotonic counters unless noted)
    pub callbacks: AtomicU64,
    pub frames: AtomicU64,
    pub cb_ns_sum: AtomicU64,
    /// Max callback duration, ns.
    pub cb_ns_max: AtomicU64,
    /// Max callback duration as permille of its buffer period.
    pub cb_permille_max: AtomicU64,
    pub cb_hist: Hist,
    /// Underrun detectors (see `core.rs`).
    pub xrun_starved: AtomicU64,
    pub xrun_late: AtomicU64,
    pub xrun_overrun: AtomicU64,
    /// Largest gap between callback starts, ns.
    pub max_gap_ns: AtomicU64,
    /// Frames per callback: last, min, max (= buffer capacity).
    pub cb_frames_last: AtomicU32,
    pub cb_frames_min: AtomicU32,
    pub cb_frames_max: AtomicU32,
    /// Playback lead (ns) of the last callback.
    pub ahead_ns: AtomicU64,
    /// Song snapshots applied, and the worst delay from dequeue to the frame
    /// tick that uses it (ns) and from send to dequeue (ns).
    pub edits_applied: AtomicU64,
    pub edit_to_tick_max_ns: AtomicU64,
    pub send_to_engine_max_ns: AtomicU64,
    pub commands: AtomicU64,
    /// Frame ticks run by the driver.
    pub driver_ticks: AtomicU64,
    /// Stream (re)builds, and the current device error (0 = none).
    pub generation: AtomicU32,
    pub device_error: AtomicU32,
    pub stream_running: AtomicBool,
    /// The audio thread's OS id (for per-thread cycle counters).
    pub audio_tid: AtomicU32,
    /// Stream callbacks that found the engine core missing (during rebuilds).
    pub silent_callbacks: AtomicU64,
}

pub mod device_error {
    pub const NONE: u32 = 0;
    /// The device went away and no replacement exists yet.
    pub const NO_DEVICE: u32 = 1;
    /// The default device changed or the stream was invalidated; rebuilding.
    pub const REBUILDING: u32 = 2;
    /// Building a stream failed.
    pub const BUILD_FAILED: u32 = 3;
}

impl Shared {
    pub fn publish_position(&self, p: &PositionSnapshot) {
        let s = &self.pos;
        let v = s.seq.load(Ordering::Relaxed);
        s.seq.store(v.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        s.ticks.store(p.ticks.to_bits(), Ordering::Relaxed);
        s.tpf.store(p.ticks_per_frame.to_bits(), Ordering::Relaxed);
        s.loop_end.store(p.loop_end.to_bits(), Ordering::Relaxed);
        s.playing.store(p.playing, Ordering::Relaxed);
        s.callback_ns.store(p.callback_ns, Ordering::Relaxed);
        s.latency_ns.store(p.latency_ns, Ordering::Relaxed);
        s.sample_rate.store(p.sample_rate, Ordering::Relaxed);
        s.seq.store(v.wrapping_add(2), Ordering::Release);
    }

    /// `None` until the first callback.
    pub fn position(&self) -> Option<PositionSnapshot> {
        let s = &self.pos;
        loop {
            let a = s.seq.load(Ordering::Acquire);
            if a & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }
            let p = PositionSnapshot {
                ticks: f64::from_bits(s.ticks.load(Ordering::Relaxed)),
                ticks_per_frame: f64::from_bits(s.tpf.load(Ordering::Relaxed)),
                loop_end: f64::from_bits(s.loop_end.load(Ordering::Relaxed)),
                playing: s.playing.load(Ordering::Relaxed),
                callback_ns: s.callback_ns.load(Ordering::Relaxed),
                latency_ns: s.latency_ns.load(Ordering::Relaxed),
                sample_rate: s.sample_rate.load(Ordering::Relaxed),
            };
            fence(Ordering::Acquire);
            if s.seq.load(Ordering::Relaxed) == a {
                return if a == 0 { None } else { Some(p) };
            }
        }
    }

    /// Max-merges per-lane levels (non-negative f32 bit patterns order like
    /// the floats).
    pub fn merge_levels(&self, l: &[f32; LANES]) {
        for (a, v) in self.levels.iter().zip(l) {
            a.fetch_max(v.max(0.0).to_bits(), Ordering::Relaxed);
        }
    }

    /// Takes (and clears) the levels accumulated since the last call.
    pub fn take_levels(&self) -> [f32; LANES] {
        let mut r = [0f32; LANES];
        for (o, a) in r.iter_mut().zip(&self.levels) {
            *o = f32::from_bits(a.swap(0, Ordering::Relaxed));
        }
        r
    }

    pub fn record_callback(&self, dur_ns: u64, period_ns: u64) {
        self.callbacks.fetch_add(1, Ordering::Relaxed);
        self.cb_ns_sum.fetch_add(dur_ns, Ordering::Relaxed);
        self.cb_ns_max.fetch_max(dur_ns, Ordering::Relaxed);
        let permille = dur_ns * 1000 / period_ns.max(1);
        self.cb_permille_max.fetch_max(permille, Ordering::Relaxed);
        let b = ((permille / HIST_STEP_PERMILLE) as usize).min(HIST_BUCKETS - 1);
        self.cb_hist[b].fetch_add(1, Ordering::Relaxed);
    }

    /// Callback cost summary: (count, mean %, p99 %, max %) of the buffer period.
    pub fn callback_cost(&self) -> (u64, f64, f64, f64) {
        let hist: Vec<u64> = self.cb_hist.iter().map(|a| a.load(Ordering::Relaxed) as u64).collect();
        let n: u64 = hist.iter().sum();
        let mut mean = 0.0;
        let mut p99 = 0.0;
        if n > 0 {
            let mut acc = 0;
            let mut p99_set = false;
            for (i, &c) in hist.iter().enumerate() {
                let pct = (i as f64 + 0.5) * HIST_STEP_PERMILLE as f64 / 10.0;
                mean += pct * c as f64;
                acc += c;
                if !p99_set && acc * 100 >= n * 99 {
                    // Upper edge of the bucket: conservative.
                    p99 = (i as f64 + 1.0) * HIST_STEP_PERMILLE as f64 / 10.0;
                    p99_set = true;
                }
            }
            mean /= n as f64;
        }
        let max = self.cb_permille_max.load(Ordering::Relaxed) as f64 / 10.0;
        (n, mean, p99, max)
    }

    pub fn underruns(&self) -> u64 {
        self.xrun_starved
            .load(Ordering::Relaxed)
            .max(self.xrun_late.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seqlock_roundtrip() {
        let s = Shared::default();
        assert_eq!(s.position(), None);
        let p = PositionSnapshot {
            ticks: 123.5,
            ticks_per_frame: 0.05,
            loop_end: 1000.0,
            playing: true,
            callback_ns: 42,
            latency_ns: 7,
            sample_rate: 48_000,
        };
        s.publish_position(&p);
        assert_eq!(s.position(), Some(p));
    }

    #[test]
    fn levels_max_merge() {
        let s = Shared::default();
        let mut a = [0f32; LANES];
        a[2] = 0.5;
        s.merge_levels(&a);
        a[2] = 0.25;
        a[3] = 1.0;
        s.merge_levels(&a);
        let t = s.take_levels();
        assert_eq!((t[2], t[3]), (0.5, 1.0));
        assert_eq!(s.take_levels(), [0.0; LANES]);
    }

    #[test]
    fn histogram_percentiles() {
        let s = Shared::default();
        for _ in 0..990 {
            s.record_callback(100_000, 10_000_000); // 1%
        }
        for _ in 0..10 {
            s.record_callback(3_000_000, 10_000_000); // 30%
        }
        let (n, mean, p99, max) = s.callback_cost();
        assert_eq!(n, 1000);
        assert!((p99 - 1.0).abs() < 0.6, "{p99}");
        assert!((max - 30.0).abs() < 0.01);
        assert!(mean > 1.0 && mean < 2.0);
    }
}
