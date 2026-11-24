//! Lightweight profiling: per-view draw spans, process CPU, and an optional
//! per-frame log (`CHIPDAW_FRAMELOG=<path>`). Costs a couple of `Instant::now()`
//! calls per instrumented draw.

use std::cell::RefCell;
use std::io::Write;

use vizia::prelude::{Duration, FrameTimings, Instant};

/// Custom views whose `draw` is timed individually.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Span {
    Arrangement,
    Ruler,
    Meters,
    Lights,
    Dials,
    Fields,
    Glyphs,
    Badges,
    Overlay,
}

pub const SPANS: [(Span, &str); 9] = [
    (Span::Arrangement, "arrange"),
    (Span::Ruler, "ruler"),
    (Span::Meters, "meters"),
    (Span::Lights, "lights"),
    (Span::Dials, "dials"),
    (Span::Fields, "fields"),
    (Span::Glyphs, "glyphs"),
    (Span::Badges, "badges"),
    (Span::Overlay, "overlay"),
];

thread_local! {
    static ACC: RefCell<[Duration; SPANS.len()]> = const { RefCell::new([Duration::ZERO; SPANS.len()]) };
}

/// Times the enclosing scope and adds it to `span`'s per-frame total.
pub struct Scope {
    span: Span,
    start: Instant,
}

pub fn scope(span: Span) -> Scope {
    Scope {
        span,
        start: Instant::now(),
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let d = self.start.elapsed();
        ACC.with(|a| a.borrow_mut()[self.span as usize] += d);
    }
}

/// Returns and resets the accumulated span times.
pub fn take_spans() -> [Duration; SPANS.len()] {
    ACC.with(|a| std::mem::replace(&mut *a.borrow_mut(), [Duration::ZERO; SPANS.len()]))
}

/// Process CPU time (user + kernel, all threads), if available on this platform.
pub fn process_cpu_time() -> Option<Duration> {
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default)]
        struct FileTime {
            lo: u32,
            hi: u32,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> isize;
            fn GetProcessTimes(
                h: isize,
                c: *mut FileTime,
                e: *mut FileTime,
                k: *mut FileTime,
                u: *mut FileTime,
            ) -> i32;
        }
        let (mut c, mut e, mut k, mut u) = Default::default();
        // SAFETY: valid out-pointers to plain-old-data; pseudo-handle needs no closing.
        let ok = unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u) };
        if ok == 0 {
            return None;
        }
        let ticks = |f: &FileTime| ((f.hi as u64) << 32) | f.lo as u64;
        // FILETIME is in 100 ns units.
        Some(Duration::from_nanos((ticks(&k) + ticks(&u)) * 100))
    }
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn clock_gettime_nsec_np(clock_id: u32) -> u64;
        }
        /// `CLOCK_PROCESS_CPUTIME_ID` in `<_time.h>`.
        const CLOCK_PROCESS_CPUTIME_ID: u32 = 12;
        // SAFETY: plain libc call with a valid clock id; returns 0 on error.
        let ns = unsafe { clock_gettime_nsec_np(CLOCK_PROCESS_CPUTIME_ID) };
        (ns != 0).then(|| Duration::from_nanos(ns))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        None
    }
}

/// Prints the GL vendor, renderer and version of the current context to
/// stderr, once. Call it from a `View::draw` (the window's context is current
/// there). Confirms at runtime which GL implementation the backend landed on.
pub fn log_gl_info_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if let Some(info) = gl_info() {
            eprintln!("chipdaw: {info}");
        }
    });
}

fn gl_info() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "OpenGL", kind = "framework")]
        unsafe extern "C" {
            fn glGetString(name: u32) -> *const std::ffi::c_char;
        }
        const GL_VENDOR: u32 = 0x1F00;
        const GL_RENDERER: u32 = 0x1F01;
        const GL_VERSION: u32 = 0x1F02;
        let get = |name| {
            // SAFETY: called with a current GL context; glGetString returns a
            // static NUL-terminated string or null.
            let p = unsafe { glGetString(name) };
            (!p.is_null())
                .then(|| unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned())
        };
        Some(format!(
            "GL_VENDOR={:?} GL_RENDERER={:?} GL_VERSION={:?}",
            get(GL_VENDOR)?,
            get(GL_RENDERER)?,
            get(GL_VERSION)?
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Smoothed process CPU %, sampled about twice a second.
#[derive(Default)]
pub struct CpuMeter {
    last: Option<(Instant, Duration)>,
    pub percent: f32,
}

impl CpuMeter {
    const PERIOD: Duration = Duration::from_millis(500);

    pub fn update(&mut self) {
        let now = Instant::now();
        let Some(cpu) = process_cpu_time() else {
            return;
        };
        match self.last {
            Some((t, c)) if now - t >= Self::PERIOD => {
                let pct = (cpu - c).as_secs_f32() / (now - t).as_secs_f32() * 100.0;
                self.percent = if self.percent == 0.0 {
                    pct
                } else {
                    self.percent * 0.6 + pct * 0.4
                };
                self.last = Some((now, cpu));
            }
            None => self.last = Some((now, cpu)),
            _ => {}
        }
    }
}

/// Optional CSV frame log, enabled with `CHIPDAW_FRAMELOG=<path>`.
pub struct FrameLog {
    out: Option<std::io::BufWriter<std::fs::File>>,
    t0: Instant,
    last_frame: u64,
    lines: u32,
}

impl FrameLog {
    pub fn from_env() -> Self {
        let out = std::env::var_os("CHIPDAW_FRAMELOG").and_then(|p| std::fs::File::create(p).ok()).map(|f| {
            let mut w = std::io::BufWriter::new(f);
            let _ = writeln!(
                w,
                "frame,t_ms,interval_ms,events_us,style_us,layout_us,make_current_us,draw_us,flush_us,present_us,custom_us,playhead_ticks"
            );
            w
        });
        Self {
            out,
            t0: Instant::now(),
            last_frame: 0,
            lines: 0,
        }
    }

    pub fn log(&mut self, ft: &FrameTimings, custom: Duration, playhead: f64) {
        let Some(w) = self.out.as_mut() else { return };
        if ft.frame_count == self.last_frame {
            return;
        }
        self.last_frame = ft.frame_count;
        let us = |d: Duration| d.as_micros();
        let t = ft
            .presented_at
            .map(|p| (p - self.t0).as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        let iv = ft.interval.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0);
        let _ = writeln!(
            w,
            "{},{:.3},{:.3},{},{},{},{},{},{},{},{},{:.1}",
            ft.frame_count,
            t,
            iv,
            us(ft.events),
            us(ft.style),
            us(ft.layout),
            us(ft.make_current),
            us(ft.draw),
            us(ft.flush),
            us(ft.present),
            us(custom),
            playhead
        );
        self.lines += 1;
        if self.lines.is_multiple_of(30) {
            let _ = w.flush();
        }
    }
}
