//! Probe: what does WASAPI shared mode grant through cpal, and what does a
//! stall look like in the callback timestamps? Prints per-buffer-size stats.
//!
//!   cargo run --release -p engine --example probe

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default, Clone, Copy, Debug)]
struct Rec {
    t_ns: u64,
    frames: u32,
    ahead_ns: u64,
}

fn main() {
    let host = cpal::default_host();
    let dev = host.default_output_device().expect("no output device");
    println!("host: {:?}", host.id());
    println!("device: {:?}", dev.description().map(|d| d.name().to_string()));
    let def = dev.default_output_config().expect("default config");
    println!("default config: {:?}", def);
    for c in dev.supported_output_configs().unwrap() {
        println!("  supported: {:?}", c);
    }

    if std::env::args().any(|a| a == "--sweep") {
        sweep(&dev);
        return;
    }
    for req in [Some(128u32), Some(256), Some(512), None] {
        run_config(&dev, req);
    }
}

/// Loopback dropout times (s since `epoch`) of a 19 kHz pilot.
fn loopback(epoch: Instant) -> (cpal::Stream, Arc<Mutex<Vec<f64>>>) {
    let dev = cpal::default_host().default_output_device().unwrap();
    let cfg = dev.default_output_config().unwrap();
    let ch = cfg.channels() as usize;
    let k = (2.0 * std::f32::consts::PI * 19_000.0 / 48_000.0).cos() * 2.0;
    let out = Arc::new(Mutex::new(Vec::new()));
    let o2 = out.clone();
    let (mut s1, mut s2, mut n, mut avg, mut b, mut low) = (0f32, 0f32, 0usize, 0f32, 0u64, false);
    let st = dev
        .build_input_stream(
            cfg.into(),
            move |d: &[f32], _: &cpal::InputCallbackInfo| {
                for f in d.chunks(ch) {
                    let s0 = f[0] + k * s1 - s2;
                    s2 = s1;
                    s1 = s0;
                    n += 1;
                    if n == 128 {
                        let amp = (s1 * s1 + s2 * s2 - k * s1 * s2).max(0.0).sqrt() / 64.0;
                        b += 1;
                        let is_low = b > 200 && amp < 0.3 * avg;
                        if is_low && !low {
                            o2.lock().unwrap().push(epoch.elapsed().as_secs_f64());
                        }
                        low = is_low;
                        if !is_low {
                            avg = if b == 1 { amp } else { avg * 0.98 + amp * 0.02 };
                        }
                        s1 = 0.0;
                        s2 = 0.0;
                        n = 0;
                    }
                }
            },
            |e| eprintln!("loopback: {e}"),
            None,
        )
        .unwrap();
    st.play().unwrap();
    (st, out)
}

/// Injects stalls of increasing length (at the END of a callback, like heavy
/// work) and prints what each one looks like next to the loopback verdict.
fn sweep(dev: &cpal::Device) {
    let config = cpal::StreamConfig { channels: 2, sample_rate: 48_000, buffer_size: cpal::BufferSize::Default };
    let epoch = Instant::now();
    let (_lb, drops) = loopback(epoch);
    let recs = Arc::new(Mutex::new(Vec::<Rec>::with_capacity(20000)));
    let stall_ms = Arc::new(AtomicU64::new(0));
    let (r2, s2) = (recs.clone(), stall_ms.clone());
    let mut phase = 0f32;
    let stream = dev
        .build_output_stream(
            config,
            move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                let now = epoch.elapsed().as_nanos() as u64;
                let ts = info.timestamp();
                let ahead = ts.playback.duration_since(ts.callback);
                for f in data.chunks_mut(2) {
                    phase = (phase + 19_000.0 / 48_000.0).fract();
                    let v = (phase * std::f32::consts::TAU).sin() * 0.01;
                    f[0] = v;
                    f[1] = v;
                }
                if let Ok(mut r) = r2.try_lock() {
                    r.push(Rec { t_ns: now, frames: (data.len() / 2) as u32, ahead_ns: ahead.as_nanos() as u64 });
                }
                let ms = s2.swap(0, Ordering::Relaxed);
                if ms > 0 {
                    let t = Instant::now();
                    while t.elapsed() < Duration::from_micros(ms) {}
                }
            },
            |e| eprintln!("stream error: {e}"),
            None,
        )
        .unwrap();
    stream.play().unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let stalls_us: Vec<u64> = (16..=30).map(|ms| ms * 1000).collect();
    let mut at = Vec::new();
    for &us in &stalls_us {
        at.push(epoch.elapsed().as_secs_f64());
        stall_ms.store(us, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(1200));
    }
    drop(stream);
    let r = recs.lock().unwrap();
    let d = drops.lock().unwrap();
    println!("stall_ms  gap_ms  next_frames  ahead_before  ahead_after1  ahead_after2  loopback");
    for (i, &us) in stalls_us.iter().enumerate() {
        let t = (at[i] * 1e9) as u64;
        let j = r.iter().position(|x| x.t_ns > t).unwrap();
        // The stall happens in callback j; its effect shows at j+1.
        let gap = (r[j + 1].t_ns - r[j].t_ns) as f64 / 1e6;
        let dropped = d.iter().any(|&x| x > at[i] && x < at[i] + 0.5);
        println!(
            "{:8.0}  {:6.2}  {:11}  {:12.2}  {:12.2}  {:12.2}  {}",
            us as f64 / 1000.0,
            gap,
            r[j + 1].frames,
            r[j].ahead_ns as f64 / 1e6,
            r[j + 1].ahead_ns as f64 / 1e6,
            r[j + 2].ahead_ns as f64 / 1e6,
            if dropped { "DROPOUT" } else { "clean" }
        );
    }
    println!("all loopback dropouts: {:?}", d.iter().map(|x| format!("{x:.2}")).collect::<Vec<_>>());
}

fn run_config(dev: &cpal::Device, req: Option<u32>) {
    {
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: 48_000,
            buffer_size: match req {
                Some(n) => cpal::BufferSize::Fixed(n),
                None => cpal::BufferSize::Default,
            },
        };
        let recs = Arc::new(Mutex::new(Vec::<Rec>::with_capacity(4000)));
        let stall_at = Arc::new(AtomicU64::new(u64::MAX));
        let epoch = Instant::now();
        let r2 = recs.clone();
        let s2 = stall_at.clone();
        let mut phase = 0f32;
        let stream = dev
            .build_output_stream(
                config,
                move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                    let now = epoch.elapsed().as_nanos() as u64;
                    let ts = info.timestamp();
                    let ahead = ts.playback.duration_since(ts.callback);
                    // Quiet 440 Hz so a glitch would be audible.
                    for f in data.chunks_mut(2) {
                        phase += 440.0 / 48_000.0;
                        if phase > 1.0 {
                            phase -= 1.0;
                        }
                        let v = (phase * std::f32::consts::TAU).sin() * 0.02;
                        f[0] = v;
                        f[1] = v;
                    }
                    if let Ok(mut r) = r2.try_lock() {
                        if r.len() < r.capacity() {
                            r.push(Rec {
                                t_ns: now,
                                frames: (data.len() / 2) as u32,
                                ahead_ns: ahead.as_nanos() as u64,
                            });
                        }
                    }
                    // Injected stall (probe only): busy-wait 40 ms once.
                    if now >= s2.load(Ordering::Relaxed) {
                        s2.store(u64::MAX, Ordering::Relaxed);
                        let t = Instant::now();
                        while t.elapsed() < Duration::from_millis(40) {}
                    }
                },
                |e| eprintln!("stream error: {e}"),
                None,
            )
            .expect("build stream");
        stream.play().unwrap();
        std::thread::sleep(Duration::from_millis(1500));
        stall_at.store(epoch.elapsed().as_nanos() as u64, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(1500));
        drop(stream);
        let r = recs.lock().unwrap();
        let skip = 20.min(r.len());
        let r = &r[skip..];
        let frames: Vec<u32> = r.iter().map(|x| x.frames).collect();
        let gaps: Vec<f64> = r.windows(2).map(|w| (w[1].t_ns - w[0].t_ns) as f64 / 1e6).collect();
        let aheads: Vec<f64> = r.iter().map(|x| x.ahead_ns as f64 / 1e6).collect();
        let mn = |v: &[f64]| v.iter().cloned().fold(f64::MAX, f64::min);
        let mx = |v: &[f64]| v.iter().cloned().fold(0.0, f64::max);
        println!(
            "\nrequested {:?}: {} callbacks; frames per callback min {} max {} (mode {}); gap ms min {:.2} max {:.2}; ahead ms min {:.2} max {:.2}",
            req,
            r.len(),
            frames.iter().min().unwrap(),
            frames.iter().max().unwrap(),
            mode(&frames),
            mn(&gaps),
            mx(&gaps),
            mn(&aheads),
            mx(&aheads)
        );
        // Around the stall: print a window of records.
        if let Some(i) = gaps.iter().position(|&g| g > 30.0) {
            println!("  around the 40 ms stall (gap ms / frames / ahead ms):");
            for j in i.saturating_sub(3)..(i + 6).min(gaps.len()) {
                println!("    {:7.2}  {:5}  {:6.2}", gaps[j], r[j + 1].frames, aheads[j + 1]);
            }
        }
        // Slack: frames delivered vs wall time after each callback.
        let f0 = r[0].t_ns as f64;
        let mut delivered = 0f64;
        let slack: Vec<f64> = r
            .iter()
            .map(|x| {
                delivered += x.frames as f64;
                delivered / 48.0 - (x.t_ns as f64 - f0) / 1e6
            })
            .collect();
        let pre = &slack[..slack.len() / 2 - 5];
        let post = &slack[slack.len() / 2 + 30..];
        println!(
            "  slack (ms delivered - ms elapsed): before stall {:.2}..{:.2}, after {:.2}..{:.2}",
            mn(pre),
            mx(pre),
            mn(post),
            mx(post)
        );
    }
}

fn mode(v: &[u32]) -> u32 {
    let mut m = std::collections::HashMap::new();
    for &x in v {
        *m.entry(x).or_insert(0) += 1;
    }
    m.into_iter().max_by_key(|e| e.1).map(|e| e.0).unwrap_or(0)
}
