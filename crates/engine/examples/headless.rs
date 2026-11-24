//! Headless engine stress harness (internal tool).
//!
//!   cargo run --release -p engine --example headless -- [options]
//!     --secs N            run time (default 30)
//!     --buffer N|default  requested buffer frames (default: device default)
//!     --ui-stall          busy-wait 250 ms on the sending thread every 2 s
//!     --edits             send a new song snapshot + param/mute changes every frame
//!     --inject MS,MS,...  inject audio-thread stalls (every 3 s; fractional ms
//!                         allowed), to calibrate detectors
//!     --every S           seconds between injected stalls (default 3)
//!     --loopback          ground truth: capture the output via WASAPI loopback
//!                         (macOS: cpal's process tap) and detect dropouts of a
//!                         19 kHz pilot tone
//!     --gain G            output gain (default 1.0; the app's stress runs use 0.3)
//!     --mute              mute all eight channels (pilot only)
//!     --pilot             mix the pilot without in-process loopback (for an
//!                         external capture such as tapmon on macOS)
//!     --block N           loopback analysis block in frames (default 128)
//!
//! The "UI" here is the main thread running at ~60 Hz.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use engine::{ClipInput, Command, Engine, EngineConfig, ParamId, SongSnapshot};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[global_allocator]
static ALLOC: engine::alloc_check::CheckingAllocator = engine::alloc_check::CheckingAllocator;

const PILOT_HZ: f32 = 19_000.0;
const PPQ: u32 = 960;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 32) as u32
    }
}

/// A busy 32-bar song on all 8 lanes: 25 clips per lane (200 clips).
fn song(rng: &mut Rng, version: u64) -> SongSnapshot {
    type Clip = (usize, u32, u32, Vec<(u32, u32, u8)>);
    let mut clips: Vec<Clip> = Vec::new();
    for lane in 0..8 {
        let mut t = 0;
        for _ in 0..25 {
            let len = PPQ * (2 + rng.next() % 4);
            let mut notes = Vec::new();
            let mut n = 0;
            while n < len {
                let l = PPQ / 4 * (1 + rng.next() % 3);
                let base = match lane {
                    2 => 36,
                    3 => 40,
                    4 => 48,
                    7 => 33,
                    _ => 60,
                };
                notes.push((n, l.min(len - n), (base + rng.next() % 24) as u8));
                n += l;
            }
            clips.push((lane, t, len, notes));
            t += len + PPQ / 2 * (rng.next() % 2);
        }
    }
    SongSnapshot::build(
        clips.iter().map(|(lane, s, l, n)| ClipInput { lane: *lane, start: *s, len: *l, notes: n }),
        version,
    )
}

struct Loopback {
    _stream: cpal::Stream,
    dropouts: Arc<Mutex<Vec<f64>>>,
    blocks: Arc<AtomicU64>,
}

fn start_loopback(epoch: Instant, block: usize) -> Loopback {
    let host = cpal::default_host();
    let dev = host.default_output_device().expect("output device");
    let cfg = dev.default_output_config().expect("config");
    let ch = cfg.channels() as usize;
    let sr = cfg.sample_rate() as f32;
    let dropouts = Arc::new(Mutex::new(Vec::new()));
    let blocks = Arc::new(AtomicU64::new(0));
    let (d2, b2) = (dropouts.clone(), blocks.clone());
    // Goertzel over `block`-frame blocks (128 = 2.7 ms at 48 kHz).
    let block_len = block;
    let k = (2.0 * std::f32::consts::PI * PILOT_HZ / sr).cos() * 2.0;
    let (mut s1, mut s2, mut n) = (0f32, 0f32, 0usize);
    let mut level_avg = 0f32;
    let mut low_run = 0u32;
    let stream = dev
        .build_input_stream(
            cfg.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for f in data.chunks(ch) {
                    let x = f[0];
                    let s0 = x + k * s1 - s2;
                    s2 = s1;
                    s1 = s0;
                    n += 1;
                    if n == block_len {
                        let p = s1 * s1 + s2 * s2 - k * s1 * s2;
                        let amp = p.max(0.0).sqrt() * 2.0 / block_len as f32;
                        let b = b2.fetch_add(1, Ordering::Relaxed);
                        if b > 200 {
                            // Dropout: a block below 30% of the running level.
                            if amp < 0.3 * level_avg {
                                low_run += 1;
                                if low_run == 1 {
                                    if let Ok(mut d) = d2.try_lock() {
                                        d.push(epoch.elapsed().as_secs_f64());
                                    }
                                }
                            } else {
                                low_run = 0;
                            }
                        }
                        if amp >= 0.3 * level_avg || b <= 200 {
                            level_avg = if b == 0 { amp } else { level_avg * 0.98 + amp * 0.02 };
                        }
                        s1 = 0.0;
                        s2 = 0.0;
                        n = 0;
                    }
                }
            },
            |e| eprintln!("loopback error: {e}"),
            None,
        )
        .expect("loopback stream (WASAPI loopback on the output device)");
    stream.play().unwrap();
    Loopback {
        _stream: stream,
        dropouts,
        blocks,
    }
}

/// Records the detector counters and takes (resets) the largest gap.
fn snap(e: &Engine, v: &mut Vec<[u64; 4]>) {
    let sh = e.shared();
    v.push([
        sh.overloads.load(Ordering::Relaxed),
        sh.xrun_late.load(Ordering::Relaxed),
        sh.xrun_overrun.load(Ordering::Relaxed),
        sh.max_gap_ns.swap(0, Ordering::Relaxed),
    ]);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let has = |k: &str| args.iter().any(|a| a == k);
    let secs: f64 = get("--secs").and_then(|s| s.parse().ok()).unwrap_or(30.0);
    let buffer = get("--buffer").and_then(|s| s.parse().ok());
    let inject: Vec<f64> = get("--inject")
        .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
        .unwrap_or_default();

    let gain: Option<f32> = get("--gain").and_then(|s| s.parse().ok());
    let block: usize = get("--block").and_then(|s| s.parse().ok()).unwrap_or(128);
    let mut e = Engine::start(EngineConfig {
        buffer_frames: buffer,
        ..Default::default()
    });
    let epoch = Instant::now();
    let lb = has("--loopback").then(|| start_loopback(epoch, block));
    let mut rng = Rng(0x1234_5678_9ABC);
    let mut version = 1;
    e.send_song(song(&mut rng, version));
    e.send(Command::Bpm(150.0));
    if let Some(g) = gain {
        e.send(Command::Gain(g));
    }
    if has("--mute") {
        e.send(Command::Mute(0xFF));
    }
    if lb.is_some() || has("--pilot") {
        e.send(Command::TestPilot(PILOT_HZ));
    }
    e.send(Command::Play);

    let t0 = Instant::now();
    let mut next_stall = 2.0;
    let every: f64 = get("--every").and_then(|s| s.parse().ok()).unwrap_or(3.0);
    let mut next_inject = every;
    let mut inject_i = 0;
    let mut injected: Vec<(f64, f64)> = Vec::new();
    // Detector counters at each injection: (overloads, late, overrun, max gap ns).
    let mut per_stall: Vec<[u64; 4]> = Vec::new();
    let mut frames = 0u64;
    let mut edits = 0u64;
    let mut wait_max = 0f64;
    while t0.elapsed().as_secs_f64() < secs {
        let now = t0.elapsed().as_secs_f64();
        // ~60 Hz "UI" frame.
        std::thread::sleep(Duration::from_micros(16_600));
        frames += 1;
        e.collect_garbage();
        if has("--edits") {
            version += 1;
            e.send_song(song(&mut rng, version));
            let lane = (rng.next() % 8) as u8;
            e.send(Command::Param { lane, id: ParamId::Volume, value: (rng.next() % 16) as i8 });
            e.send(Command::Param { lane: 0, id: ParamId::Duty, value: (rng.next() % 4) as i8 });
            if frames.is_multiple_of(30) {
                e.send(Command::Mute((rng.next() & 0xFF) as u8 & 0x7F));
            }
            if frames.is_multiple_of(47) {
                e.send(Command::Solo(0));
            }
            edits += 1;
        }
        if has("--ui-stall") && now >= next_stall {
            next_stall += 2.0;
            let s = Instant::now();
            while s.elapsed() < Duration::from_millis(250) {
                std::hint::spin_loop();
            }
        }
        if !inject.is_empty() && now >= next_inject {
            next_inject += every;
            let ms = inject[inject_i % inject.len()];
            inject_i += 1;
            snap(&e, &mut per_stall);
            injected.push((epoch.elapsed().as_secs_f64(), ms));
            e.send(Command::TestStall((ms * 1000.0) as u32));
        }
        wait_max = wait_max.max(e.pending() as f64);
    }

    if !injected.is_empty() {
        std::thread::sleep(Duration::from_millis(300));
        snap(&e, &mut per_stall);
    }
    let sh = e.shared();
    let (n, mean, p99, max) = sh.callback_cost();
    println!("device: {}", e.device_info());
    println!(
        "callbacks {n}, frames/callback {}..{}, playback lead {:.2} ms",
        sh.cb_frames_min.load(Ordering::Relaxed),
        sh.cb_frames_max.load(Ordering::Relaxed),
        sh.ahead_ns.load(Ordering::Relaxed) as f64 / 1e6
    );
    println!("callback cost (% of buffer period): mean {mean:.3}, p99 {p99:.2}, max {max:.2}");
    println!(
        "underrun detectors: starved {}, late {}, overrun {}; max gap {:.2} ms",
        sh.xrun_starved.load(Ordering::Relaxed),
        sh.xrun_late.load(Ordering::Relaxed),
        sh.xrun_overrun.load(Ordering::Relaxed),
        sh.max_gap_ns.load(Ordering::Relaxed) as f64 / 1e6
    );
    println!(
        "edits applied {} (sent {edits}+1), send->engine max {:.2} ms, dequeue->frame tick max {:.2} ms, dropped commands {}",
        sh.edits_applied.load(Ordering::Relaxed),
        sh.send_to_engine_max_ns.load(Ordering::Relaxed) as f64 / 1e6,
        sh.edit_to_tick_max_ns.load(Ordering::Relaxed) as f64 / 1e6,
        e.dropped_commands()
    );
    println!(
        "driver ticks {} ({:.4} Hz), alloc violations {}",
        sh.driver_ticks.load(Ordering::Relaxed),
        sh.driver_ticks.load(Ordering::Relaxed) as f64 / (sh.frames.load(Ordering::Relaxed) as f64 / 48_000.0),
        engine::alloc_check::VIOLATIONS.load(Ordering::Relaxed)
    );
    println!(
        "stream builds {} (rebuilds {}), silent callbacks {}, overload reports {}, device error {}",
        sh.generation.load(Ordering::Relaxed),
        sh.generation.load(Ordering::Relaxed).saturating_sub(1),
        sh.silent_callbacks.load(Ordering::Relaxed),
        sh.overloads.load(Ordering::Relaxed),
        sh.device_error.load(Ordering::Relaxed)
    );
    if !injected.is_empty() {
        println!("per stall: ms, overloads, late, overrun, largest gap ms");
        for (i, (_, ms)) in injected.iter().enumerate() {
            let (a, b) = (per_stall[i], per_stall[i + 1]);
            println!(
                "  stall {ms:7.3}  {:2}  {:2}  {:2}  {:7.3}",
                b[0] - a[0],
                b[1] - a[1],
                b[2] - a[2],
                b[3] as f64 / 1e6
            );
        }
        println!("injected stalls (t s, ms): {:?}", injected.iter().map(|(t, m)| (format!("{t:.2}"), *m)).collect::<Vec<_>>());
    }
    if let Some(lb) = lb {
        let d = lb.dropouts.lock().unwrap().clone();
        println!(
            "loopback: {} blocks analysed, {} dropouts at t = {:?}",
            lb.blocks.load(Ordering::Relaxed),
            d.len(),
            d.iter().map(|t| format!("{t:.2}")).collect::<Vec<_>>()
        );
    }
}
