//! Records the default output through cpal's loopback (WASAPI loopback on
//! Windows, a CoreAudio process tap on macOS) to a 32-bit float WAV, for
//! checking what the dropout detector actually sees.
//!
//!   cargo run --release -p engine --example loopcap -- OUT.wav [SECS]
//!
//! Prints the capture config, every callback's frame count and the gaps
//! between capture timestamps that don't match the frames delivered.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "loopcap.wav".into());
    let secs: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5.0);
    let dev = cpal::default_host().default_output_device().expect("output device");
    let cfg = dev.default_output_config().expect("config");
    println!("capturing {:?} with {:?}", dev.description().map(|d| d.name().to_string()), cfg);
    let ch = cfg.channels() as usize;
    let sr = cfg.sample_rate();
    let data = Arc::new(Mutex::new(Vec::<f32>::new()));
    // (frames, capture timestamp ns) per callback
    let cbs = Arc::new(Mutex::new(Vec::<(usize, u128)>::new()));
    let (d2, c2) = (data.clone(), cbs.clone());
    let stream = dev
        .build_input_stream(
            cfg.into(),
            move |x: &[f32], info: &cpal::InputCallbackInfo| {
                d2.lock().unwrap().extend_from_slice(x);
                c2.lock().unwrap().push((x.len() / ch, info.timestamp().capture.as_nanos()));
            },
            |e| eprintln!("capture error: {e}"),
            None,
        )
        .expect("loopback stream");
    stream.play().unwrap();
    std::thread::sleep(Duration::from_secs_f64(secs));
    drop(stream);
    let d = data.lock().unwrap();
    let c = cbs.lock().unwrap();
    let mut sizes: Vec<usize> = c.iter().map(|x| x.0).collect();
    sizes.sort_unstable();
    sizes.dedup();
    println!("{} callbacks, frame counts {:?}, {} frames total", c.len(), sizes, d.len() / ch);
    let mut odd = 0;
    for w in c.windows(2) {
        let expect = w[0].0 as f64 / sr as f64 * 1e9;
        let got = (w[1].1 as f64) - (w[0].1 as f64);
        if (got - expect).abs() > expect * 0.1 {
            odd += 1;
            if odd <= 10 {
                println!("  capture time step {:.3} ms for {} frames", got / 1e6, w[0].0);
            }
        }
    }
    println!("{odd} callbacks whose capture-time step doesn't match their frame count (>10%)");
    let mut f = std::io::BufWriter::new(std::fs::File::create(&out).unwrap());
    let bytes = (d.len() * 4) as u32;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + bytes).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&3u16.to_le_bytes()).unwrap(); // IEEE float
    f.write_all(&(ch as u16).to_le_bytes()).unwrap();
    f.write_all(&sr.to_le_bytes()).unwrap();
    f.write_all(&(sr * 4 * ch as u32).to_le_bytes()).unwrap();
    f.write_all(&((4 * ch) as u16).to_le_bytes()).unwrap();
    f.write_all(&32u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&bytes.to_le_bytes()).unwrap();
    for v in d.iter() {
        f.write_all(&v.to_le_bytes()).unwrap();
    }
    println!("wrote {out}");
}
