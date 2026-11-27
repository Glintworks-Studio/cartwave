//! Stress-test hooks (ENGINE.md §5). All off unless their env var is set,
//! and only in dev mode (`CARTWAVE_DEV=1` or a debug build; see `dev.rs`).
//!
//! - `CARTWAVE_AUTO_EDIT=1`  test B: continuous random edits through the same
//!   model paths as the mouse (drag with live overlap resolution, drop,
//!   resize, duplicate, delete), dial turns, mute/solo toggles.
//! - `CARTWAVE_UI_STALL=1`   test C: busy-wait 250 ms on the UI thread every 2 s.
//! - `CARTWAVE_AUDIOLOG=path` one CSV line of engine stats per second, plus a
//!   summary at exit.
//! - `CARTWAVE_PILOT=1`      mix a 23 kHz pilot (`CARTWAVE_PILOT_HZ` to change) and watch it via WASAPI loopback
//!   (ground-truth dropout count).
//! - `CARTWAVE_AVSYNC=1`     test G: 1-bar loop, music muted, a click at mid-bar;
//!   loopback click times vs the presented playhead crossing.
//! - `CARTWAVE_QUIT_AFTER=secs` exit (after the summary) when the time is up.
//! - `CARTWAVE_SONG=heavy` start on the macro worst case (every lane runs all macros).
//! - `CARTWAVE_STRESS=1` start on the 200-clip set; `CARTWAVE_AUTOPLAY=1` start playing.
//! - `CARTWAVE_BUFFER=frames` requested buffer size; `CARTWAVE_FILTER=off|nes|440|fc`.

use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::mock::Rng;
use crate::state::*;
use engine::cpu::Counts;
use engine::testkit::Loopback;
use engine::Engine;

/// The host clock now, in ns (`mach_absolute_time`, the clock behind
/// `Instant` on macOS). `None` elsewhere.
fn host_ns_now() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn mach_absolute_time() -> u64;
            fn mach_timebase_info(info: *mut [u32; 2]) -> i32;
        }
        let mut tb = [0u32; 2];
        // SAFETY: plain libc calls with a valid out-pointer.
        let t = unsafe {
            mach_timebase_info(&mut tb);
            mach_absolute_time()
        };
        Some((t as u128 * tb[0] as u128 / tb[1].max(1) as u128) as u64)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn zero_counts() -> Counts {
    // Zeros (not None) so `add` accumulates on platforms that measure them.
    Counts { cycles: 0, instructions: Some(0), energy_nj: Some(0), p_ns: Some(0), e_ns: Some(0) }
}

/// `c / n / 1000`, or 0.
fn per(c: u64, n: u64) -> f64 {
    if n > 0 {
        c as f64 / n as f64 / 1e3
    } else {
        0.0
    }
}

fn env_on(k: &str) -> bool {
    crate::dev::var(k).is_some_and(|v| v != "0" && !v.is_empty())
}

/// Marker for the A/V sync test: mid-bar of a one-bar loop.
pub const AV_LOOP_TICKS: f64 = TICKS_PER_BAR as f64;
pub const AV_MARKER_TICKS: f64 = TICKS_PER_BAR as f64 / 2.0;

pub struct Tests {
    pub auto_edit: bool,
    pub ui_stall: bool,
    pub avsync: bool,
    pub pilot: bool,
    pub stress: bool,
    /// `CARTWAVE_SONG=heavy`: start on the macro worst case (`mock::heavy_arrangement`).
    pub heavy: bool,
    pub autoplay: bool,
    pub buffer: Option<u32>,
    pub filter: Option<engine::FilterMode>,
    log: Option<File>,
    log_path: Option<String>,
    quit_after: Option<f64>,
    start: Instant,
    next_stall: f64,
    next_log: f64,
    rng: Rng,
    drag: Option<(u32, u32)>,
    pub loopback: Option<Loopback>,
    /// (presented at, playhead shown) for presented frames (A/V test).
    shown: Vec<(Instant, f64)>,
    pending_shown: Option<(u64, f64)>,
    pub edits: u64,
    pub stalls: u64,
    pub gain: Option<f32>,
    audio_clock: Option<(u64, engine::cpu::ThreadClock)>,
    /// (audio counters, callbacks) and (ui counters, frames) at the last log line.
    last_audio: Option<(Counts, u64)>,
    last_ui: Option<(Counts, u64)>,
    last_proc: Option<(Counts, Instant)>,
    /// Totals since the first log line: audio counters / callbacks, ui
    /// counters / frames, process counters / seconds.
    acc_audio: (Counts, u64),
    acc_ui: (Counts, u64),
    acc_proc: (Counts, f64),
    pub ui_frames: u64,
    /// Frames where the playhead was integrated locally instead of read from
    /// the audio clock (no stream, or a command in flight), since the last line.
    pub playhead_local_frames: u64,
    /// Refresh rate of the display the window is on (for the display-move test).
    pub display_hz: Option<f64>,
    last_tick: Option<Instant>,
    /// Largest gap between UI pump ticks since the last line (App Nap, stalls).
    tick_gap_max: Duration,
}

impl Tests {
    pub fn from_env() -> Self {
        let log_path = crate::dev::var("CARTWAVE_AUDIOLOG");
        let log = log_path.as_ref().and_then(|p| File::create(p).ok());
        let filter = crate::dev::var("CARTWAVE_FILTER").and_then(|v| match v.as_str() {
            "off" => Some(engine::FilterMode::Off),
            "nes" => Some(engine::FilterMode::Nes),
            "440" => Some(engine::FilterMode::NesWith440),
            "fc" => Some(engine::FilterMode::Famicom),
            _ => None,
        });
        let mut t = Self {
            auto_edit: env_on("CARTWAVE_AUTO_EDIT"),
            ui_stall: env_on("CARTWAVE_UI_STALL"),
            avsync: env_on("CARTWAVE_AVSYNC"),
            pilot: env_on("CARTWAVE_PILOT"),
            stress: env_on("CARTWAVE_STRESS"),
            heavy: crate::dev::var("CARTWAVE_SONG").is_some_and(|v| v == "heavy"),
            autoplay: env_on("CARTWAVE_AUTOPLAY"),
            buffer: crate::dev::var("CARTWAVE_BUFFER").and_then(|v| v.parse().ok()),
            filter,
            log,
            log_path,
            quit_after: crate::dev::var("CARTWAVE_QUIT_AFTER").and_then(|v| v.parse().ok()),
            start: Instant::now(),
            next_stall: 2.0,
            next_log: 1.0,
            rng: Rng::new(0xED17),
            drag: None,
            loopback: None,
            shown: Vec::new(),
            pending_shown: None,
            edits: 0,
            stalls: 0,
            gain: crate::dev::var("CARTWAVE_GAIN").and_then(|v| v.parse().ok()),
            audio_clock: None,
            last_audio: None,
            last_ui: None,
            last_proc: None,
            acc_audio: (zero_counts(), 0),
            acc_ui: (zero_counts(), 0),
            acc_proc: (zero_counts(), 0.0),
            ui_frames: 0,
            playhead_local_frames: 0,
            display_hz: None,
            last_tick: None,
            tick_gap_max: Duration::ZERO,
        };
        // F-02: every harness proves it reached its scenario from this line
        // (stderr), since a release build without CARTWAVE_DEV ignores the hooks.
        if crate::dev::enabled() {
            let on: Vec<&str> = [
                ("auto_edit", t.auto_edit),
                ("ui_stall", t.ui_stall),
                ("avsync", t.avsync),
                ("pilot", t.pilot),
                ("stress", t.stress),
                ("heavy", t.heavy),
                ("autoplay", t.autoplay),
                ("audiolog", t.log.is_some()),
                ("quit_after", t.quit_after.is_some()),
            ]
            .into_iter()
            .filter_map(|(n, b)| b.then_some(n))
            .collect();
            eprintln!("dev: mode on; test hooks: [{}]; buffer {:?}; filter {:?}", on.join(", "), t.buffer, t.filter);
        }
        if let Some(f) = t.log.as_mut() {
            let _ = writeln!(
                f,
                "t_s,callbacks,starved,late,overrun,cb_mean_pct,cb_p99_pct,cb_max_pct,edits_applied,ui_edits,edit_to_tick_max_ms,send_to_engine_max_ms,driver_ticks,alloc_violations,pending,dropped,device_error,generation,frames_per_cb,ahead_ms,loopback_dropouts,ui_stalls,audio_kcycles_per_cb,ui_kcycles_per_frame,audio_kinstr_per_cb,audio_uj_per_cb,audio_p_share,ui_kinstr_per_frame,ui_uj_per_frame,ui_p_share,proc_minstr_per_s,proc_mw,proc_p_share,overloads,silent_callbacks,ui_tick_gap_max_ms,playhead_local_frames,display_hz,max_gap_ms"
            );
        }
        t
    }

    pub fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Test C: busy-wait 250 ms every 2 s. Called at the start of every pump
    /// tick, so it also records the largest gap between ticks.
    pub fn maybe_stall(&mut self) {
        let now = Instant::now();
        if let Some(t) = self.last_tick {
            self.tick_gap_max = self.tick_gap_max.max(now - t);
        }
        self.last_tick = Some(now);
        if self.ui_stall && self.elapsed() >= self.next_stall {
            self.next_stall += 2.0;
            self.stalls += 1;
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(250) {
                std::hint::spin_loop();
            }
        }
    }

    /// Test B: one random edit per call, via the same model operations as the
    /// mouse. Returns a dial change to apply (lane, slot value) if any.
    pub fn auto_edit(&mut self, a: &mut Arrangement) -> Option<(usize, engine::ParamId, i32)> {
        if !self.auto_edit || a.clips.is_empty() {
            return None;
        }
        self.edits += 1;
        let r = &mut self.rng;
        let six = (PPQ / 4) as i64;
        // An ongoing drag: move the dragged clip a little each frame (live
        // overlap resolution in the snapshot), then drop it.
        if let Some((id, left)) = self.drag {
            if let Some(c) = a.clip_mut(id) {
                let d = (r.range(0, 5) as i64 - 2) * six;
                c.start_ticks = (c.start_ticks as i64 + d).max(0) as u32;
            }
            if left == 0 {
                let w = std::mem::take(&mut a.editing);
                a.resolve_overlaps(&w);
                self.drag = None;
            } else {
                self.drag = Some((id, left - 1));
            }
            return None;
        }
        let pick = |a: &Arrangement, r: &mut Rng| a.clips[r.range(0, a.clips.len() as u32) as usize].id;
        match r.range(0, 100) {
            // Start a drag (also changes lane sometimes).
            0..=24 => {
                let id = pick(a, r);
                if r.f32() < 0.3 {
                    if let Some(c) = a.clip_mut(id) {
                        c.lane = r.range(0, 8) as usize;
                    }
                }
                a.raise(&[id]);
                a.editing = vec![id];
                self.drag = Some((id, r.range(3, 20)));
            }
            // Instant move + drop.
            25..=44 => {
                let id = pick(a, r);
                if let Some(c) = a.clip_mut(id) {
                    let d = (r.range(0, 17) as i64 - 8) * six;
                    c.start_ticks = (c.start_ticks as i64 + d).max(0) as u32;
                }
                a.resolve_overlaps(&[id]);
            }
            // Resize the right edge (keep a minimum length).
            45..=59 => {
                let id = pick(a, r);
                if let Some(c) = a.clip_mut(id) {
                    let d = (r.range(0, 9) as i64 - 4) * six;
                    c.len_ticks = (c.len_ticks as i64 + d).max(MIN_CLIP_TICKS as i64) as u32;
                }
                a.resolve_overlaps(&[id]);
            }
            // Duplicate somewhere else (keeps the count near 200).
            60..=69 => {
                if let Some(mut c) = a.clip(pick(a, r)).cloned() {
                    c.id = a.next_id;
                    a.next_id += 1;
                    c.start_ticks = r.range(0, 30) * TICKS_PER_BAR;
                    c.rev = crate::state::Rev::next();
                    let id = c.id;
                    a.push_clip(c);
                    a.resolve_overlaps(&[id]);
                }
            }
            // Delete (only while there are plenty).
            70..=77 => {
                if a.clips.len() > 150 {
                    let id = pick(a, r);
                    a.clips.retain(|c| c.id != id);
                    a.selection.retain(|s| *s != id);
                }
            }
            // Dial turn.
            _ => {
                let lane = r.range(0, 8) as usize;
                let dials = lane_dials(lane);
                if !dials.is_empty() {
                    let (pid, spec) = dials[r.range(0, dials.len() as u32) as usize];
                    let v = spec.min as i32 + r.range(0, spec.steps + 1) as i32;
                    return Some((lane, pid, v));
                }
            }
        }
        None
    }

    /// Random mute/solo toggles for test B (a few per second).
    pub fn auto_mute(&mut self) -> Option<(bool, usize)> {
        if !self.auto_edit {
            return None;
        }
        let r = &mut self.rng;
        if r.range(0, 20) == 0 {
            Some((r.f32() < 0.7, r.range(0, 8) as usize))
        } else {
            None
        }
    }

    /// A/V test: remembers the playhead prepared for the next presented frame,
    /// and pairs it with that frame's present time once it is known.
    pub fn av_frame(&mut self, frame_count: u64, presented_at: Option<Instant>, prepared: f64) {
        if !self.avsync {
            return;
        }
        if let (Some((n, ticks)), Some(at)) = (self.pending_shown, presented_at) {
            if frame_count > n {
                self.shown.push((at, ticks));
                self.pending_shown = None;
            }
        }
        if self.pending_shown.is_none() {
            self.pending_shown = Some((frame_count, prepared));
        }
    }

    /// Once per second: a CSV line. At `quit_after`: the summary, then exit.
    pub fn periodic(&mut self, e: &Engine) {
        let t = self.elapsed();
        if t < self.next_log {
            return;
        }
        self.next_log += 1.0;
        use std::sync::atomic::Ordering::Relaxed;
        let sh = e.shared();
        let (n, mean, p99, max) = sh.callback_cost();
        let lb = self.loopback.as_ref().map_or(0, |l| l.dropouts.load(Relaxed));
        // Per-thread counters: audio per callback, UI per presented frame.
        let tid = sh.audio_tid.load(Relaxed);
        if tid != 0 && self.audio_clock.as_ref().is_none_or(|(t, _)| *t != tid) {
            self.audio_clock = engine::cpu::ThreadClock::open(tid).map(|c| (tid, c));
            self.last_audio = None;
        }
        let mut audio = (zero_counts(), 0u64);
        if let Some(c) = self.audio_clock.as_ref().and_then(|(_, c)| c.counts()) {
            if let Some((c0, n0)) = self.last_audio {
                if n > n0 {
                    audio = (c.since(&c0), n - n0);
                    self.acc_audio.0.add(&audio.0);
                    self.acc_audio.1 += audio.1;
                }
            }
            self.last_audio = Some((c, n));
        }
        let mut ui = (zero_counts(), 0u64);
        if let Some(c) = engine::cpu::current_thread_counts() {
            if let Some((c0, f0)) = self.last_ui {
                if self.ui_frames > f0 {
                    ui = (c.since(&c0), self.ui_frames - f0);
                    self.acc_ui.0.add(&ui.0);
                    self.acc_ui.1 += ui.1;
                }
            }
            self.last_ui = Some((c, self.ui_frames));
        }
        let mut proc = (zero_counts(), 0.0f64);
        if let Some(c) = engine::cpu::process_counts() {
            let now = Instant::now();
            if let Some((c0, t0)) = self.last_proc {
                proc = (c.since(&c0), (now - t0).as_secs_f64());
                self.acc_proc.0.add(&proc.0);
                self.acc_proc.1 += proc.1;
            }
            self.last_proc = Some((c, now));
        }
        let audio_k = per(audio.0.cycles, audio.1);
        let ui_k = per(ui.0.cycles, ui.1);
        let opt = |v: Option<f64>, scale: f64, prec: usize| v.map_or(String::new(), |v| format!("{:.*}", prec, v * scale));
        let extra = format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            opt(audio.0.instructions.map(|i| i as f64 / audio.1.max(1) as f64), 1e-3, 1),
            opt(audio.0.energy_nj.map(|e| e as f64 / audio.1.max(1) as f64), 1e-3, 2),
            opt(audio.0.p_share(), 1.0, 3),
            opt(ui.0.instructions.map(|i| i as f64 / ui.1.max(1) as f64), 1e-3, 0),
            opt(ui.0.energy_nj.map(|e| e as f64 / ui.1.max(1) as f64), 1e-3, 1),
            opt(ui.0.p_share(), 1.0, 3),
            opt(proc.0.instructions.map(|i| i as f64 / proc.1.max(1e-9)), 1e-6, 1),
            opt(proc.0.energy_nj.map(|e| e as f64 / proc.1.max(1e-9)), 1e-6, 1),
            opt(proc.0.p_share(), 1.0, 3),
            sh.overloads.load(Relaxed),
            sh.silent_callbacks.load(Relaxed),
        );
        let extra = format!(
            "{extra},{:.1},{},{},{:.2}",
            self.tick_gap_max.as_secs_f64() * 1e3,
            self.playhead_local_frames,
            self.display_hz.map_or(String::new(), |h| format!("{h:.3}")),
            sh.max_gap_ns.load(Relaxed) as f64 / 1e6,
        );
        self.tick_gap_max = Duration::ZERO;
        self.playhead_local_frames = 0;
        if let Some(f) = self.log.as_mut() {
            let _ = writeln!(
                f,
                "{t:.1},{n},{},{},{},{mean:.3},{p99:.2},{max:.2},{},{},{:.2},{:.2},{},{},{},{},{},{},{},{:.2},{lb},{},{audio_k:.1},{ui_k:.1},{extra}",
                sh.xrun_starved.load(Relaxed),
                sh.xrun_late.load(Relaxed),
                sh.xrun_overrun.load(Relaxed),
                sh.edits_applied.load(Relaxed),
                self.edits,
                sh.edit_to_tick_max_ns.load(Relaxed) as f64 / 1e6,
                sh.send_to_engine_max_ns.load(Relaxed) as f64 / 1e6,
                sh.driver_ticks.load(Relaxed),
                engine::alloc_check::VIOLATIONS.load(Relaxed),
                e.pending(),
                e.dropped_commands(),
                sh.device_error.load(Relaxed),
                sh.generation.load(Relaxed),
                sh.cb_frames_last.load(Relaxed),
                sh.ahead_ns.load(Relaxed) as f64 / 1e6,
                self.stalls,
            );
        }
        if self.quit_after.is_some_and(|q| t >= q) {
            self.summary(e);
            std::process::exit(0);
        }
    }

    fn summary(&mut self, e: &Engine) {
        use std::sync::atomic::Ordering::Relaxed;
        let sh = e.shared();
        let (n, mean, p99, max) = sh.callback_cost();
        let mut s = format!(
            "# summary: {:.0} s, device \"{}\"\n# callbacks {n}, frames/cb {}..{}, cost mean {mean:.3}% p99 {p99:.2}% max {max:.2}% of period\n# underruns: starved {} late {} (overruns {}), max callback gap {:.2} ms\n# edits applied {}, edit->frame tick max {:.2} ms, send->engine max {:.2} ms, dropped commands {}\n# alloc violations {}, ui stalls {}, device rebuilds {}\n",
            self.elapsed(),
            e.device_info(),
            sh.cb_frames_min.load(Relaxed),
            sh.cb_frames_max.load(Relaxed),
            sh.xrun_starved.load(Relaxed),
            sh.xrun_late.load(Relaxed),
            sh.xrun_overrun.load(Relaxed),
            sh.max_gap_ns.load(Relaxed) as f64 / 1e6,
            sh.edits_applied.load(Relaxed),
            sh.edit_to_tick_max_ns.load(Relaxed) as f64 / 1e6,
            sh.send_to_engine_max_ns.load(Relaxed) as f64 / 1e6,
            e.dropped_commands(),
            engine::alloc_check::VIOLATIONS.load(Relaxed),
            self.stalls,
            sh.generation.load(Relaxed).saturating_sub(1),
        );
        let (a, na) = self.acc_audio;
        let (u, nu) = self.acc_ui;
        let (p, secs) = self.acc_proc;
        s += &format!(
            "# thread cycles: audio {:.1} k/callback, ui {:.1} k/presented frame\n",
            per(a.cycles, na),
            per(u.cycles, nu)
        );
        if let (Some(ai), Some(ui)) = (a.instructions, u.instructions) {
            let pct = |v: Option<f64>| v.map_or("?".into(), |v| format!("{:.1}%", v * 100.0));
            s += &format!(
                "# audio thread: {:.1} k instr/callback, {:.2} uJ/callback, P-core share {}; ui thread: {:.0} k instr/frame, {:.1} uJ/frame, P-core share {}\n",
                per(ai, na),
                per(a.energy_nj.unwrap_or(0), na),
                pct(a.p_share()),
                per(ui, nu),
                per(u.energy_nj.unwrap_or(0), nu),
                pct(u.p_share()),
            );
            let s_ = secs.max(1e-9);
            s += &format!(
                "# process: {:.1} M instr/s, {:.1} mW, P-core share {}, {:.1} % of a core; audio thread {:.2} M instr/s, {:.2} mW\n",
                p.instructions.unwrap_or(0) as f64 / s_ / 1e6,
                p.energy_nj.unwrap_or(0) as f64 / s_ / 1e6,
                pct(p.p_share()),
                (p.p_ns.unwrap_or(0) + p.e_ns.unwrap_or(0)) as f64 / s_ / 1e7,
                ai as f64 / s_ / 1e6,
                a.energy_nj.unwrap_or(0) as f64 / s_ / 1e6,
            );
        }
        s += &format!(
            "# overloads (CoreAudio kAudioDeviceProcessorOverload via cpal Xrun): {}; silent callbacks {}\n",
            sh.overloads.load(Relaxed),
            sh.silent_callbacks.load(Relaxed)
        );
        // Callback cost histogram (0.1%-of-period buckets; the last is >= 20%).
        let hist: Vec<String> = sh
            .cb_hist
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let c = c.load(Relaxed);
                (c > 0).then(|| format!("{i}:{c}"))
            })
            .collect();
        s += &format!("# cb_hist_permille: {}\n", hist.join(" "));
        if let Some(now) = host_ns_now() {
            // For lining up events captured outside the app (tapmon).
            let start = now - self.start.elapsed().as_nanos() as u64;
            s += &format!("# window_host_ns: {start} {now}\n");
        }
        if self.avsync {
            s += &self.write_crossings();
        }
        if let Some(lb) = &self.loopback {
            s += &format!(
                "# loopback: {} blocks, {} pilot dropouts\n",
                lb.blocks.load(Relaxed),
                lb.dropouts.load(Relaxed)
            );
            if self.avsync {
                s += &self.av_report(lb);
            }
        }
        if let Some(f) = self.log.as_mut() {
            let _ = f.write_all(s.as_bytes());
            let _ = f.flush();
        }
        eprint!("{s}");
    }

    /// When the presented playhead crossed the A/V marker.
    fn crossings(&self) -> Vec<Instant> {
        let mut crossings = Vec::new();
        for w in self.shown.windows(2) {
            let ((t0, p0), (t1, p1)) = (w[0], w[1]);
            if p0 < AV_MARKER_TICKS && p1 >= AV_MARKER_TICKS && p1 > p0 {
                let f = (AV_MARKER_TICKS - p0) / (p1 - p0);
                crossings.push(t0 + (t1 - t0).mul_f64(f));
            }
        }
        crossings
    }

    /// Writes the marker crossings as host-clock ns (`mach_absolute_time` in
    /// ns, the clock of `Instant` and of tapmon's events) to
    /// `<CARTWAVE_AUDIOLOG>.crossings.csv`, for pairing with clicks captured
    /// outside the app (macOS).
    fn write_crossings(&self) -> String {
        let (Some(path), Some(now_host)) = (self.log_path.as_ref(), host_ns_now()) else {
            return String::new();
        };
        let now = Instant::now();
        let mut out = String::from("crossing_host_ns\n");
        let c = self.crossings();
        for t in &c {
            let back = now.saturating_duration_since(*t).as_nanos() as u64;
            out += &format!("{}\n", now_host - back);
        }
        let file = format!("{path}.crossings.csv");
        match std::fs::write(&file, out) {
            Ok(()) => format!("# avsync: {} marker crossings written to {file}\n", c.len()),
            Err(e) => format!("# avsync: can't write {file}: {e}\n"),
        }
    }

    /// Pairs each visual marker crossing with the nearest audible click.
    fn av_report(&self, lb: &Loopback) -> String {
        let crossings = self.crossings();
        let clicks = lb.clicks.lock().map(|c| c.clone()).unwrap_or_default();
        let mut offs: Vec<f64> = Vec::new();
        for v in &crossings {
            let best = clicks
                .iter()
                .map(|a| {
                    if *a >= *v {
                        (*a - *v).as_secs_f64() * 1e3
                    } else {
                        -((*v - *a).as_secs_f64() * 1e3)
                    }
                })
                .min_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap());
            if let Some(o) = best.filter(|o| o.abs() < 400.0) {
                offs.push(o);
            }
        }
        if offs.is_empty() {
            return format!(
                "# avsync: no pairs ({} crossings, {} clicks)\n",
                crossings.len(),
                clicks.len()
            );
        }
        let n = offs.len() as f64;
        let mean = offs.iter().sum::<f64>() / n;
        let sd = (offs.iter().map(|o| (o - mean).powi(2)).sum::<f64>() / n).sqrt();
        let mn = offs.iter().cloned().fold(f64::MAX, f64::min);
        let mx = offs.iter().cloned().fold(f64::MIN, f64::max);
        format!(
            "# avsync: {} pairs ({} crossings, {} clicks); audio - visual (presented) offset ms: mean {mean:+.2} sd {sd:.2} min {mn:+.2} max {mx:+.2}\n",
            offs.len(),
            crossings.len(),
            clicks.len()
        )
    }
}
