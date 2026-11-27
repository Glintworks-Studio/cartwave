//! Probe for the B4C audio gate (internal tool): is the loopback capture the
//! same samples we handed the device? Plays a known pseudo-random signal on
//! the default output, captures it through cpal's loopback, finds the offset
//! and reports the residual after a best-fit gain.
//!
//!   cargo run --release -p engine --example tapprobe -- [SECS] [LEVEL]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn pn(state: &mut u32) -> f32 {
    // xorshift32 -> [-1, 1)
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1u32 << 23) as f32 - 1.0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let secs: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(4.0);
    let level: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let host = cpal::default_host();
    let dev = host.default_output_device().expect("output device");
    let cfg = dev.default_output_config().expect("config");
    println!("device {:?} config {:?}", dev.description().map(|d| d.name().to_string()), cfg);
    let ch = cfg.channels() as usize;
    let sent = Arc::new(Mutex::new(Vec::<f32>::with_capacity(48_000 * 10)));
    let got = Arc::new(Mutex::new(Vec::<f32>::with_capacity(48_000 * 10)));
    let (s2, g2) = (sent.clone(), got.clone());
    let mut st = 0x1234_5678u32;
    let out = dev
        .build_output_stream(
            cfg.clone().into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut s = s2.lock().unwrap();
                for f in data.chunks_mut(ch) {
                    let v = pn(&mut st) * level;
                    f.fill(v);
                    s.push(v);
                }
            },
            |e| eprintln!("out error {e}"),
            None,
        )
        .expect("output");
    let inp = dev
        .build_input_stream(
            cfg.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut g = g2.lock().unwrap();
                for f in data.chunks(ch) {
                    g.push(f[0]);
                }
            },
            |e| eprintln!("in error {e}"),
            None,
        )
        .expect("loopback");
    inp.play().unwrap();
    out.play().unwrap();
    std::thread::sleep(Duration::from_secs_f64(secs));
    drop(out);
    drop(inp);
    let sent = sent.lock().unwrap();
    let got = got.lock().unwrap();
    println!("sent {} frames, captured {}", sent.len(), got.len());
    // Find where sent[24000..24064] appears in the capture.
    let probe = &sent[24_000..24_064];
    let mut best = (f64::MAX, 0usize);
    for c in 0..got.len().saturating_sub(64) {
        let e: f64 = probe.iter().zip(&got[c..c + 64]).map(|(a, b)| ((a - b) as f64).powi(2)).sum();
        if e < best.0 {
            best = (e, c);
        }
    }
    let off = best.1 as i64 - 24_000;
    println!("offset capture - sent = {off} frames, probe residual {:.3e}", best.0);
    let (mut sxy, mut sxx, mut exact, mut n) = (0f64, 0f64, 0usize, 0usize);
    let mut maxd = 0f32;
    for (i, &s) in sent.iter().enumerate() {
        let c = i as i64 + off;
        if c < 0 || c as usize >= got.len() || i < 4800 {
            continue;
        }
        let g = got[c as usize];
        sxy += (s * g) as f64;
        sxx += (s * s) as f64;
        if s.to_bits() == g.to_bits() {
            exact += 1;
        }
        maxd = maxd.max((s - g).abs());
        n += 1;
    }
    let gain = sxy / sxx;
    let mut e = 0f64;
    for (i, &s) in sent.iter().enumerate() {
        let c = i as i64 + off;
        if c < 0 || c as usize >= got.len() || i < 4800 {
            continue;
        }
        e += (got[c as usize] as f64 - gain * s as f64).powi(2);
    }
    println!(
        "compared {n} frames: bit-exact {exact} ({:.4}%), max |diff| {maxd:.3e}, gain {gain:.6}, residual {:.1} dB re signal",
        100.0 * exact as f64 / n.max(1) as f64,
        10.0 * (e / (gain * gain * sxx)).log10()
    );
}
