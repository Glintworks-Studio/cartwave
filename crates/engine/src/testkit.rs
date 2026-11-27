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

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
}

impl Loopback {
    /// Captures the default output device (WASAPI loopback).
    pub fn start(pilot_hz: Option<f32>) -> Result<Self, String> {
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
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            dropouts,
            dropout_times,
            clicks,
            blocks,
        })
    }
}
