//! Test instrumentation (not a user feature): WASAPI loopback capture of the
//! output, as ground truth for dropouts and for A/V sync.
//!
//! On macOS this in-process capture is refused for output devices that also
//! have inputs: cpal then records the physical inputs, not the output (it only
//! builds a process tap for output-only devices). The macOS ground truth is
//! the external `docs/engine/tools/macos/tapmon` (ENGINE-macos.md §3).
//!
//! - Dropouts: the engine mixes a quiet pilot tone (`Command::TestPilot`); a
//!   Goertzel filter over 128-frame blocks flags blocks where the pilot falls
//!   below 30% of its running level.
//! - Clicks: onsets of the full-scale test click (`Command::TestClickAt`),
//!   time-stamped as `Instant`s via cpal's capture timestamps, so the UI can
//!   compare them with its own presented-frame times.
//! - Reference match (Batch 4c, ENGINE.md §7a): the engine copies the first
//!   output channel, exactly as handed to the device, into a wait-free
//!   [`OutputTap`]; a checker thread aligns the loopback capture with that
//!   copy sample for sample ([`Matcher`]). WASAPI shared-mode loopback is
//!   bit-exact (probe: `examples/tapprobe.rs`), so the music can't mask
//!   anything. A dropout is silence the device inserted: the capture's
//!   offset from the copy jumps forward by the gap's length in frames. A
//!   quiet pseudo-random marker (`Command::TestMarker`, -60 dBFS) keeps the
//!   alignment unique through silence and held notes.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Level of the reference-match marker (about -60 dBFS before the output gain).
pub const MARKER_AMP: f32 = 1.0 / 1024.0;

/// The marker's next sample (xorshift32, never zero), in [-MARKER_AMP, MARKER_AMP).
#[inline]
pub fn marker_next(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    ((x >> 8) as f32 / (1u32 << 23) as f32 - 1.0) * MARKER_AMP
}

/// A copy of the first output channel, written by the audio thread (atomic
/// stores into a preallocated ring: no allocation, no locks) and read by the
/// checker. Frames are numbered from the first one written.
pub struct OutputTap {
    buf: Box<[AtomicU32]>,
    mask: u64,
    written: AtomicU64,
}

impl OutputTap {
    /// `frames` is rounded up to a power of two.
    pub fn new(frames: usize) -> Self {
        let n = frames.next_power_of_two();
        Self {
            buf: (0..n).map(|_| AtomicU32::new(0)).collect(),
            mask: n as u64 - 1,
            written: AtomicU64::new(0),
        }
    }

    pub fn capacity(&self) -> u64 {
        self.mask + 1
    }

    pub fn written(&self) -> u64 {
        self.written.load(Ordering::Acquire)
    }

    /// Audio thread: appends the first channel of an interleaved buffer.
    #[inline]
    pub fn write(&self, out: &[f32], channels: usize) {
        let w = self.written.load(Ordering::Relaxed);
        let mut i = 0u64;
        for f in out.chunks_exact(channels.max(1)) {
            self.buf[((w + i) & self.mask) as usize].store(f[0].to_bits(), Ordering::Relaxed);
            i += 1;
        }
        self.written.store(w + i, Ordering::Release);
    }

    /// Reader: appends frames `from..` to `dst`. Returns the next frame to
    /// read and how many frames were lost (overwritten before they were read).
    pub fn read(&self, from: u64, dst: &mut VecDeque<f32>) -> (u64, u64) {
        let w = self.written.load(Ordering::Acquire);
        // Keep a quarter of the ring as margin against the writer lapping us.
        let keep = self.capacity() * 3 / 4;
        let start = from.max(w.saturating_sub(keep));
        for i in start..w {
            dst.push_back(f32::from_bits(self.buf[(i & self.mask) as usize].load(Ordering::Relaxed)));
        }
        let mut lost = start - from;
        if self.written.load(Ordering::Acquire).saturating_sub(start) > self.capacity() {
            // Lapped while copying: the copied frames can't be trusted.
            lost += w - start;
        }
        (w, lost)
    }
}

/// What the reference match found (see [`Matcher`]).
#[derive(Clone, Debug, Default)]
pub struct MatchStats {
    /// 128-frame capture blocks equal to the engine's copy at the locked offset.
    pub matched_blocks: u64,
    /// Blocks before the first lock (the capture starts before our stream).
    pub unlocked_blocks: u64,
    /// Blocks that matched nothing (inside a gap, or foreign sound).
    pub mismatched_blocks: u64,
    /// Runs of mismatched blocks (each glitch counts once).
    pub mismatch_events: u64,
    /// Forward offset jumps: silence the device inserted (render dropouts).
    pub dropouts: u64,
    /// Backward jumps: frames missing from the capture (capture-side gaps).
    pub capture_gaps: u64,
    /// Mismatch runs that re-locked at the same offset (sound that wasn't ours).
    pub foreign: u64,
    /// Tap frames overwritten before the checker read them (invalidates the run).
    pub tap_lost: u64,
    /// Locks acquired (the first one, plus one after each mismatch run).
    pub locks: u64,
    /// (capture time in s, offset jump in frames) of every jump.
    pub jumps: Vec<(f64, i64)>,
    /// Largest forward jump, frames.
    pub max_jump: i64,
    /// Exact sample comparisons done (the proof the check ran).
    pub samples_compared: u64,
}

/// Aligns a capture with the engine's own copy of its output (Batch 4c).
pub struct Matcher {
    sr: f64,
    refb: VecDeque<f32>,
    ref_base: u64,
    ref_next: u64,
    cap: VecDeque<f32>,
    cap_base: u64,
    /// capture frame = reference frame + offset, once locked.
    offset: Option<i64>,
    in_mismatch: bool,
    mismatch_offset: i64,
    pub stats: MatchStats,
}

impl Matcher {
    pub const BLOCK: usize = 128;
    /// Frames that must match to (re)lock: with the marker, unique.
    pub const WINDOW: usize = 1024;
    /// How far a re-lock searches either side of the old offset (1 s at 48 kHz).
    pub const SEARCH: i64 = 48_000;
    /// Largest difference that still counts as equal. Loopback is bit-exact on
    /// the PC measured; this only forgives format round-off.
    pub const TOL: f32 = 1.0e-5;

    pub fn new(sample_rate: f64) -> Self {
        Self {
            sr: sample_rate,
            refb: VecDeque::new(),
            ref_base: 0,
            ref_next: 0,
            cap: VecDeque::new(),
            cap_base: 0,
            offset: None,
            in_mismatch: false,
            mismatch_offset: 0,
            stats: MatchStats::default(),
        }
    }

    /// Pulls new frames from the tap.
    pub fn pull_tap(&mut self, tap: &OutputTap) {
        let before = self.refb.len();
        let (next, lost) = tap.read(self.ref_next, &mut self.refb);
        if lost > 0 {
            self.stats.tap_lost += lost;
            // Start the reference again after the hole.
            let keep = self.refb.len() - before;
            self.refb.drain(..before);
            self.ref_base = next - keep as u64;
            self.offset = None;
        }
        self.ref_next = next;
    }

    /// Reference frames directly (tests).
    pub fn push_ref(&mut self, x: &[f32]) {
        self.refb.extend(x.iter().copied());
        self.ref_next += x.len() as u64;
    }

    pub fn push_capture(&mut self, x: &[f32]) {
        self.cap.extend(x.iter().copied());
    }

    /// Is the checker locked onto the stream now?
    pub fn locked(&self) -> bool {
        self.offset.is_some() && !self.in_mismatch
    }

    fn ref_end(&self) -> u64 {
        self.ref_base + self.refb.len() as u64
    }

    /// Does the capture from `c` (absolute) equal the reference from `r` for `n` frames?
    fn equal(&mut self, c: u64, r: u64, n: usize) -> bool {
        let (ci, ri) = ((c - self.cap_base) as usize, (r - self.ref_base) as usize);
        for k in 0..n {
            self.stats.samples_compared += 1;
            if (self.cap[ci + k] - self.refb[ri + k]).abs() > Self::TOL {
                return false;
            }
        }
        true
    }

    /// Searches reference frames `lo..hi` (absolute) for the capture window at
    /// `c`. Returns the reference frame where it starts.
    fn search(&mut self, c: u64, lo: u64, hi: u64) -> Option<u64> {
        let ci = (c - self.cap_base) as usize;
        // Silence matches any silence: lock only on a window that carries
        // the marker (or music) almost everywhere.
        let live = self.cap.range(ci..ci + Self::WINDOW).filter(|v| **v != 0.0).count();
        if live < Self::WINDOW * 7 / 8 {
            return None;
        }
        let first = self.cap[ci];
        let lo = lo.max(self.ref_base);
        let hi = hi.min(self.ref_end().saturating_sub(Self::WINDOW as u64));
        let mut r = lo;
        while r < hi {
            let ri = (r - self.ref_base) as usize;
            if (self.refb[ri] - first).abs() <= Self::TOL && self.equal(c, r, Self::WINDOW) {
                return Some(r);
            }
            r += 1;
        }
        None
    }

    fn pop_block(&mut self) {
        self.cap.drain(..Self::BLOCK);
        self.cap_base += Self::BLOCK as u64;
    }

    /// Consumes every capture block it can judge now.
    pub fn run(&mut self) {
        let b = Self::BLOCK as u64;
        let span = Self::SEARCH as u64;
        while self.cap.len() >= Self::WINDOW {
            let c = self.cap_base;
            match self.offset {
                None => {
                    if self.refb.len() < Self::WINDOW {
                        break;
                    }
                    let (lo, hi) = (self.ref_base, self.ref_end());
                    if let Some(r) = self.search(c, lo, hi) {
                        self.offset = Some(c as i64 - r as i64);
                        self.stats.locks += 1;
                        continue;
                    }
                    // The capture may be ahead of the copy: wait while the
                    // reference is younger than the search span.
                    if (self.refb.len() as u64) < span {
                        break;
                    }
                    self.stats.unlocked_blocks += 1;
                    self.pop_block();
                }
                Some(off) => {
                    let r = c as i64 - off;
                    if r < self.ref_base as i64 {
                        // The copy no longer holds it (tap lost): lock again.
                        self.offset = None;
                        continue;
                    }
                    let r = r as u64;
                    if r + b > self.ref_end() {
                        break;
                    }
                    if self.equal(c, r, Self::BLOCK) {
                        self.stats.matched_blocks += 1;
                        if self.in_mismatch {
                            // Matching again at the old offset: no frames
                            // were lost, something else was mixed in.
                            self.stats.foreign += 1;
                        }
                        self.in_mismatch = false;
                        self.pop_block();
                        continue;
                    }
                    if !self.in_mismatch {
                        self.in_mismatch = true;
                        self.stats.mismatch_events += 1;
                        self.mismatch_offset = off;
                    }
                    // Re-lock: search either side of the old offset, once the
                    // copy reaches far enough to rule out a later match.
                    if self.ref_end() < r + span + Self::WINDOW as u64 {
                        break;
                    }
                    let lo = r.saturating_sub(span);
                    if let Some(r2) = self.search(c, lo, r + span + Self::WINDOW as u64) {
                        let off2 = c as i64 - r2 as i64;
                        let jump = off2 - self.mismatch_offset;
                        if jump > 0 {
                            self.stats.dropouts += 1;
                            self.stats.max_jump = self.stats.max_jump.max(jump);
                        } else if jump < 0 {
                            self.stats.capture_gaps += 1;
                        } else {
                            self.stats.foreign += 1;
                        }
                        if jump != 0 {
                            self.stats.jumps.push((c as f64 / self.sr, jump));
                        }
                        self.stats.locks += 1;
                        self.offset = Some(off2);
                        self.in_mismatch = false;
                        continue;
                    }
                    self.stats.mismatched_blocks += 1;
                    self.pop_block();
                }
            }
        }
        // Trim the copy: keep two search spans behind the capture.
        let keep_from = match self.offset {
            Some(off) => (self.cap_base as i64 - off - 2 * Self::SEARCH).max(0) as u64,
            None => self.ref_end().saturating_sub(2 * span),
        };
        if keep_from > self.ref_base {
            let n = ((keep_from - self.ref_base) as usize).min(self.refb.len());
            self.refb.drain(..n);
            self.ref_base += n as u64;
        }
    }
}

/// 23 kHz: above the band-limited passband (21.6 kHz), so no music can mask it.
/// 19 kHz (the ENGINE.md default) is masked under the Famicom filter (docs/RUNLOG.md, A2).
pub const PILOT_HZ: f32 = 23_000.0;

/// The pilot frequency: `CARTWAVE_PILOT_HZ`, or `PILOT_HZ`. A pilot above the
/// band-limited synthesis' passband (21.6 kHz at 48 kHz) can't be masked by
/// music; with the Famicom filter (no 14 kHz low-pass) a 19 kHz pilot can
/// (ENGINE-macos.md §3).
pub fn pilot_hz() -> f32 {
    crate::env::var("CARTWAVE_PILOT_HZ")
        .and_then(|v| v.parse().ok())
        .unwrap_or(PILOT_HZ)
}

pub struct Loopback {
    _stream: cpal::Stream,
    pub dropouts: Arc<AtomicU64>,
    pub dropout_times: Arc<Mutex<Vec<Instant>>>,
    pub clicks: Arc<Mutex<Vec<Instant>>>,
    pub blocks: Arc<AtomicU64>,
    /// The reference match's results, when started with a tap.
    pub matched: Option<Arc<Mutex<MatchStats>>>,
    checker: Option<(Arc<AtomicBool>, std::thread::JoinHandle<()>)>,
}

impl Drop for Loopback {
    fn drop(&mut self) {
        if let Some((stop, h)) = self.checker.take() {
            stop.store(true, Ordering::Relaxed);
            let _ = h.join();
        }
    }
}

impl Loopback {
    /// Captures the default output device (WASAPI loopback).
    pub fn start(pilot_hz: Option<f32>) -> Result<Self, String> {
        Self::start_with(pilot_hz, None)
    }

    /// As `start`, plus the reference match against `tap` (Batch 4c): the
    /// capture's first channel goes to a checker thread.
    pub fn start_with(pilot_hz: Option<f32>, tap: Option<(Arc<OutputTap>, u32)>) -> Result<Self, String> {
        let host = cpal::default_host();
        let dev = host.default_output_device().ok_or("no output device")?;
        if cfg!(target_os = "macos") && dev.supports_input() {
            return Err("the output device also has inputs, so cpal would record them instead of the output; use tapmon".into());
        }
        let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
        let ch = cfg.channels() as usize;
        let sr = cfg.sample_rate() as f64;
        let dropouts = Arc::new(AtomicU64::new(0));
        let dropout_times = Arc::new(Mutex::new(Vec::new()));
        let clicks = Arc::new(Mutex::new(Vec::new()));
        let blocks = Arc::new(AtomicU64::new(0));
        let (d1, d2, c2, b2) = (dropouts.clone(), dropout_times.clone(), clicks.clone(), blocks.clone());
        // Reference match: the capture callback hands its first channel over
        // (a mutex on the capture thread, never on our audio thread).
        let captured = Arc::new(Mutex::new(Vec::<f32>::with_capacity(1 << 16)));
        let mut checker = None;
        let mut matched = None;
        if let Some((tap, engine_sr)) = tap {
            if engine_sr as f64 != sr {
                return Err(format!("reference match needs the engine at the device rate ({engine_sr} vs {sr} Hz)"));
            }
            let stats = Arc::new(Mutex::new(MatchStats::default()));
            let stop = Arc::new(AtomicBool::new(false));
            let (st2, stop2, cap2) = (stats.clone(), stop.clone(), captured.clone());
            let h = std::thread::Builder::new()
                .name("cartwave-loopback-match".into())
                .spawn(move || {
                    let mut m = Matcher::new(sr);
                    let mut buf = Vec::with_capacity(1 << 16);
                    while !stop2.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(20));
                        if let Ok(mut c) = cap2.lock() {
                            std::mem::swap(&mut *c, &mut buf);
                        }
                        m.push_capture(&buf);
                        buf.clear();
                        m.pull_tap(&tap);
                        m.run();
                        if let Ok(mut s) = st2.lock() {
                            s.clone_from(&m.stats);
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
            checker = Some((stop, h));
            matched = Some(stats);
        }
        let cap_on = matched.is_some();
        let cap3 = captured.clone();
        const BLOCK: usize = 128;
        let k = pilot_hz.map(|f| (2.0 * std::f64::consts::PI * f as f64 / sr).cos() * 2.0);
        let (mut s1, mut s2, mut n) = (0f64, 0f64, 0usize);
        let (mut avg, mut low) = (0f64, false);
        let mut quiet = 0usize;
        let stream = dev
            .build_input_stream(
                cfg.into(),
                move |data: &[f32], info: &cpal::InputCallbackInfo| {
                    let now = Instant::now();
                    let ts = info.timestamp();
                    // When the first frame of this buffer was captured, as an Instant.
                    let first = now
                        .checked_sub(ts.callback.duration_since(ts.capture))
                        .unwrap_or(now);
                    if cap_on {
                        if let Ok(mut c) = cap3.lock() {
                            c.extend(data.chunks(ch).map(|f| f[0]));
                        }
                    }
                    for (i, f) in data.chunks(ch).enumerate() {
                        let x = f[0] as f64;
                        // Click onset: loud after >= 50 ms of quiet. (The test
                        // mutes the music; the click is 0.9 x output gain.)
                        if x.abs() > 0.1 {
                            if quiet > (0.05 * sr) as usize {
                                if let Ok(mut c) = c2.lock() {
                                    c.push(first + Duration::from_secs_f64(i as f64 / sr));
                                }
                            }
                            quiet = 0;
                        } else {
                            quiet += 1;
                        }
                        if let Some(k) = k {
                            let s0 = x + k * s1 - s2;
                            s2 = s1;
                            s1 = s0;
                            n += 1;
                            if n == BLOCK {
                                let amp = (s1 * s1 + s2 * s2 - k * s1 * s2).max(0.0).sqrt() * 2.0
                                    / BLOCK as f64;
                                let b = b2.fetch_add(1, Ordering::Relaxed);
                                let is_low = b > 200 && amp < 0.3 * avg;
                                if is_low && !low {
                                    d1.fetch_add(1, Ordering::Relaxed);
                                    if let Ok(mut d) = d2.lock() {
                                        d.push(first + Duration::from_secs_f64(i as f64 / sr));
                                    }
                                }
                                low = is_low;
                                if !is_low {
                                    avg = if b == 0 { amp } else { avg * 0.98 + amp * 0.02 };
                                }
                                s1 = 0.0;
                                s2 = 0.0;
                                n = 0;
                            }
                        }
                    }
                },
                |_e| {},
                None,
            )
            .map_err(|e| e.to_string())
            .and_then(|s| s.play().map(|_| s).map_err(|e| e.to_string()));
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                if let Some((stop, h)) = checker {
                    stop.store(true, Ordering::Relaxed);
                    let _ = h.join();
                }
                return Err(e);
            }
        };
        Ok(Self {
            _stream: stream,
            dropouts,
            dropout_times,
            clicks,
            blocks,
            matched,
            checker,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stream like the engine's: music (a held square wave, periodic on
    /// purpose) plus the marker.
    fn stream(n: usize) -> Vec<f32> {
        let mut st = 0x9E37_79B9u32;
        (0..n)
            .map(|i| if (i / 50) % 2 == 0 { 0.2 } else { -0.2 } + marker_next(&mut st))
            .collect()
    }

    fn run(reference: &[f32], capture: &[f32]) -> MatchStats {
        let mut m = Matcher::new(48_000.0);
        // Feed in callback-sized pieces, the copy slightly ahead, as live.
        for (r, c) in reference.chunks(480).zip(capture.chunks(480)) {
            m.push_ref(r);
            m.push_capture(c);
            m.run();
        }
        m.run();
        m.stats
    }

    #[test]
    fn a_clean_stream_matches_everywhere() {
        let r = stream(48_000 * 6);
        // The capture starts with 0.3 s of silence (before our stream) and
        // then carries the copy from frame 0.
        let mut c = vec![0.0; 14_400];
        c.extend_from_slice(&r);
        let s = run(&r, &c);
        assert_eq!((s.dropouts, s.capture_gaps, s.foreign, s.mismatch_events), (0, 0, 0, 0), "{s:?}");
        assert_eq!(s.locks, 1);
        assert!(s.matched_blocks > 48_000 * 4 / 128, "{s:?}");
        assert!(s.samples_compared > 48_000 * 4);
    }

    #[test]
    fn inserted_silence_is_a_dropout_of_its_length() {
        let r = stream(48_000 * 8);
        for gap in [480usize, 1056, 4800] {
            let mut c = r[..100_000].to_vec();
            c.extend(std::iter::repeat_n(0.0, gap));
            c.extend_from_slice(&r[100_000..]);
            let s = run(&r, &c);
            assert_eq!(s.dropouts, 1, "gap {gap}: {s:?}");
            assert_eq!(s.jumps.len(), 1);
            assert_eq!(s.jumps[0].1, gap as i64, "the jump is the gap's length");
            assert_eq!(s.capture_gaps, 0);
        }
    }

    #[test]
    fn a_held_note_is_still_aligned_by_the_marker() {
        // Without the marker a dropout of exactly one period of a periodic
        // signal would re-lock at the old offset; with it, never.
        let r = stream(48_000 * 6);
        let mut c = r[..60_000].to_vec();
        c.extend(std::iter::repeat_n(0.0, 100));
        c.extend_from_slice(&r[60_000..]);
        let s = run(&r, &c);
        assert_eq!((s.dropouts, s.jumps.first().map(|j| j.1)), (1, Some(100)), "{s:?}");
    }

    #[test]
    fn missing_capture_frames_are_not_dropouts() {
        let r = stream(48_000 * 6);
        let mut c = r[..90_000].to_vec();
        c.extend_from_slice(&r[90_960..]);
        let s = run(&r, &c);
        assert_eq!((s.dropouts, s.capture_gaps), (0, 1), "{s:?}");
    }

    #[test]
    fn foreign_sound_is_reported_without_a_jump() {
        let r = stream(48_000 * 6);
        let mut c = r.clone();
        for v in &mut c[70_000..70_300] {
            *v += 0.05;
        }
        let s = run(&r, &c);
        assert_eq!((s.dropouts, s.capture_gaps, s.foreign), (0, 0, 1), "{s:?}");
    }

    #[test]
    fn silence_never_locks() {
        let r = vec![0.0; 48_000 * 3];
        let s = run(&r, &r);
        assert_eq!(s.locks, 0);
        assert_eq!(s.dropouts, 0);
    }

    #[test]
    fn the_tap_returns_what_was_written_and_reports_losses() {
        let tap = OutputTap::new(1024);
        let out: Vec<f32> = (0..600).flat_map(|i| [i as f32, -(i as f32)]).collect();
        tap.write(&out, 2);
        let mut d = VecDeque::new();
        let (next, lost) = tap.read(0, &mut d);
        assert_eq!((next, lost, d.len()), (600, 0, 600));
        assert!(d.iter().enumerate().all(|(i, v)| *v == i as f32), "the first channel");
        // The writer laps a reader that fell behind.
        tap.write(&out, 2);
        tap.write(&out, 2);
        let mut d = VecDeque::new();
        let (next, lost) = tap.read(600, &mut d);
        assert_eq!(next, 1800);
        assert!(lost > 0);
    }
}
