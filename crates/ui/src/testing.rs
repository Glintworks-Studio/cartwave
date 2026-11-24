//! Stress-test hooks (ENGINE.md §5). All off unless their env var is set.
//!
//! - `CHIPDAW_AUTO_EDIT=1`  test B: continuous random edits through the same
//!   model paths as the mouse (drag with live overlap resolution, drop,
//!   resize, duplicate, delete), dial turns, mute/solo toggles.
//! - `CHIPDAW_UI_STALL=1`   test C: busy-wait 250 ms on the UI thread every 2 s.
//! - `CHIPDAW_AUDIOLOG=path` one CSV line of engine stats per second, plus a
//!   summary at exit.
//! - `CHIPDAW_PILOT=1`      mix a 19 kHz pilot and watch it via WASAPI loopback
//!   (ground-truth dropout count).
//! - `CHIPDAW_AVSYNC=1`     test G: 1-bar loop, music muted, a click at mid-bar;
//!   loopback click times vs the presented playhead crossing.
//! - `CHIPDAW_QUIT_AFTER=secs` exit (after the summary) when the time is up.
//! - `CHIPDAW_STRESS=1` start on the 200-clip set; `CHIPDAW_AUTOPLAY=1` start playing.
//! - `CHIPDAW_BUFFER=frames` requested buffer size; `CHIPDAW_FILTER=off|nes|440|fc`.

use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::mock::Rng;
use crate::state::*;
use engine::testkit::Loopback;
use engine::Engine;

fn env_on(k: &str) -> bool {
    std::env::var(k).is_ok_and(|v| v != "0" && !v.is_empty())
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
    pub autoplay: bool,
    pub buffer: Option<u32>,
    pub filter: Option<engine::FilterMode>,
    log: Option<File>,
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
}

impl Tests {
    pub fn from_env() -> Self {
        let log = std::env::var("CHIPDAW_AUDIOLOG")
            .ok()
            .and_then(|p| File::create(p).ok());
        let filter = std::env::var("CHIPDAW_FILTER").ok().and_then(|v| match v.as_str() {
            "off" => Some(engine::FilterMode::Off),
            "nes" => Some(engine::FilterMode::Nes),
            "440" => Some(engine::FilterMode::NesWith440),
            "fc" => Some(engine::FilterMode::Famicom),
            _ => None,
        });
        let mut t = Self {
            auto_edit: env_on("CHIPDAW_AUTO_EDIT"),
            ui_stall: env_on("CHIPDAW_UI_STALL"),
            avsync: env_on("CHIPDAW_AVSYNC"),
            pilot: env_on("CHIPDAW_PILOT"),
            stress: env_on("CHIPDAW_STRESS"),
            autoplay: env_on("CHIPDAW_AUTOPLAY"),
            buffer: std::env::var("CHIPDAW_BUFFER").ok().and_then(|v| v.parse().ok()),
            filter,
            log,
            quit_after: std::env::var("CHIPDAW_QUIT_AFTER").ok().and_then(|v| v.parse().ok()),
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
        };
        if let Some(f) = t.log.as_mut() {
            let _ = writeln!(
                f,
                "t_s,callbacks,starved,late,overrun,cb_mean_pct,cb_p99_pct,cb_max_pct,edits_applied,ui_edits,edit_to_tick_max_ms,send_to_engine_max_ms,driver_ticks,alloc_violations,pending,dropped,device_error,generation,frames_per_cb,ahead_ms,loopback_dropouts,ui_stalls"
            );
        }
        t
    }

    pub fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Test C: busy-wait 250 ms every 2 s.
    pub fn maybe_stall(&mut self) {
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
                    let id = c.id;
                    a.clips.push(c);
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
        if let Some(f) = self.log.as_mut() {
            let _ = writeln!(
                f,
                "{t:.1},{n},{},{},{},{mean:.3},{p99:.2},{max:.2},{},{},{:.2},{:.2},{},{},{},{},{},{},{},{:.2},{lb},{}",
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

    /// Pairs each visual marker crossing with the nearest audible click.
    fn av_report(&self, lb: &Loopback) -> String {
        let mut crossings = Vec::new();
        for w in self.shown.windows(2) {
            let ((t0, p0), (t1, p1)) = (w[0], w[1]);
            if p0 < AV_MARKER_TICKS && p1 >= AV_MARKER_TICKS && p1 > p0 {
                let f = (AV_MARKER_TICKS - p0) / (p1 - p0);
                crossings.push(t0 + (t1 - t0).mul_f64(f));
            }
        }
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
