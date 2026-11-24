//! Offline test harness for the chip crate (internal tool, not a user feature).
//!
//!   cargo run --release -p chip --example lab -- alias     # aliasing measurements
//!   cargo run --release -p chip --example lab -- tuning    # docs/tuning.md
//!   cargo run --release -p chip --example lab -- cost      # render cost
//!   cargo run --release -p chip --example lab -- pack      # docs/audio/*.wav (chip-level part)

use chip::filter::FilterMode;
use chip::pitch::{self, Osc};
use chip::samples;
use chip::synth::Synth;
use std::f64::consts::PI;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

const FS: f64 = 48_000.0;
const CHUNK: usize = 1024;

// ---------------------------------------------------------------- rendering

type Writes = Vec<(u16, u8)>;
/// (file stem, script, seconds, description)
type PackFile = (&'static str, Vec<(f64, Writes)>, f64, &'static str);

/// Renders a register-write script: (time in seconds, writes).
fn render_script(
    script: &[(f64, Writes)],
    secs: f64,
    filter: FilterMode,
    band_limited: bool,
) -> Vec<f32> {
    let mut s = Box::new(Synth::new(FS));
    s.set_filter(filter);
    s.band_limited = band_limited;
    samples::build(&mut s.rom);
    let total = (secs * FS) as usize;
    let mut out = vec![0f32; total];
    let mut pos = 0;
    let mut events = script.iter().peekable();
    while pos < total {
        while let Some((t, w)) = events.peek() {
            if ((t * FS) as usize) <= pos {
                for &(a, v) in w {
                    s.write(a, v);
                }
                events.next();
            } else {
                break;
            }
        }
        let next_event = events.peek().map_or(total, |(t, _)| ((t * FS) as usize).max(pos + 1));
        let end = (pos + CHUNK).min(total).min(next_event);
        s.render(&mut out[pos..end]);
        pos = end;
    }
    out
}

fn init() -> Writes {
    vec![(0x4015, 0x1F), (0x4017, 0x40), (0x4001, 0x08), (0x4005, 0x08), (0x9003, 0x00)]
}

fn write_wav(path: &Path, x: &[f32]) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).expect("create wav"));
    let data_len = (x.len() * 2) as u32;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    f.write_all(&(FS as u32).to_le_bytes()).unwrap();
    f.write_all(&(FS as u32 * 2).to_le_bytes()).unwrap();
    f.write_all(&2u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();
    for &v in x {
        let s = (v.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        f.write_all(&s.to_le_bytes()).unwrap();
    }
}

// ---------------------------------------------------------------- spectrum

fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f64;
        for i in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = (ang * k as f64).sin_cos();
                let (ar, ai) = (re[i + k + len / 2], im[i + k + len / 2]);
                let tr = ar * c - ai * s;
                let ti = ar * s + ai * c;
                re[i + k + len / 2] = re[i + k] - tr;
                im[i + k + len / 2] = im[i + k] - ti;
                re[i + k] += tr;
                im[i + k] += ti;
            }
        }
        len <<= 1;
    }
}

/// Power spectrum with a 4-term Blackman-Harris window (sidelobes < -92 dB).
fn power_spectrum(x: &[f32]) -> Vec<f64> {
    let n = x.len();
    let mut re: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let t = 2.0 * PI * i as f64 / n as f64;
            let w = 0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos()
                - 0.01168 * (3.0 * t).cos();
            v as f64 * w
        })
        .collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im);
    (0..n / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect()
}

struct AliasReport {
    /// Non-harmonic power / harmonic power, 20 Hz..20 kHz, dB.
    alias_db: f64,
    /// Strongest non-harmonic bin relative to the fundamental, dB.
    worst_dbc: f64,
}

fn alias_report(x: &[f32], f0: f64) -> AliasReport {
    let p = power_spectrum(x);
    let n = x.len() as f64;
    let bin_hz = FS / n;
    let guard = 5.0 * bin_hz; // BH4 main lobe is +/-4 bins
    let fund_bin = (f0 / bin_hz).round() as usize;
    let fund = p[fund_bin.saturating_sub(4)..fund_bin + 5].iter().cloned().fold(0.0, f64::max);
    let (mut sig, mut ali, mut worst) = (0.0, 0.0, 0.0f64);
    for (k, &v) in p.iter().enumerate() {
        let f = k as f64 * bin_hz;
        if !(20.0..=20_000.0).contains(&f) {
            continue;
        }
        let h = (f / f0).round() * f0;
        if h > 0.0 && (f - h).abs() <= guard {
            sig += v;
        } else {
            ali += v;
            worst = worst.max(v);
        }
    }
    AliasReport {
        alias_db: 10.0 * (ali / sig).log10(),
        worst_dbc: 10.0 * (worst / fund).log10(),
    }
}

struct Tone {
    name: &'static str,
    f0: f64,
    writes: Writes,
}

fn tones() -> Vec<Tone> {
    let pulse = |t: u16, duty: u8| -> Writes {
        vec![
            (0x4000, (duty << 6) | 0x30 | 12),
            (0x4002, t as u8),
            (0x4003, 0x08 | (t >> 8) as u8),
        ]
    };
    let mut v = vec![
        Tone { name: "pulse 50%, t=53", f0: Osc::Pulse.frequency(53), writes: pulse(53, 2) },
        Tone { name: "pulse 12.5%, t=26", f0: Osc::Pulse.frequency(26), writes: pulse(26, 0) },
        Tone { name: "pulse 25%, t=211", f0: Osc::Pulse.frequency(211), writes: pulse(211, 1) },
        Tone {
            name: "triangle, t=26",
            f0: Osc::Triangle.frequency(26),
            writes: vec![(0x4008, 0xFF), (0x400A, 26), (0x400B, 0x08)],
        },
        Tone {
            name: "VRC6 saw, t=60, A=42",
            f0: Osc::Saw.frequency(60),
            writes: vec![(0xB000, 42), (0xB001, 60), (0xB002, 0x80)],
        },
        Tone {
            name: "VRC6 pulse 3/16, t=101",
            f0: Osc::Vrc6Pulse.frequency(101),
            writes: vec![(0x9000, 0x20 | 15), (0x9001, 101), (0x9002, 0x80)],
        },
    ];
    for t in &mut v {
        let mut w = init();
        w.append(&mut t.writes);
        t.writes = w;
    }
    v
}

fn alias() {
    const N: usize = 1 << 16;
    println!("Aliasing: non-harmonic / harmonic power in 20 Hz..20 kHz, and the strongest");
    println!("non-harmonic component relative to the fundamental. 48 kHz, {N}-point BH4 FFT,");
    println!("after 0.5 s settling.\n");
    println!("| Tone | f0 (Hz) | Naive, raw | Band-limited, raw | Naive, NES filter | Band-limited, NES filter |");
    println!("|---|---|---|---|---|---|");
    for tone in tones() {
        let mut cells = Vec::new();
        for (filter, bl) in [
            (FilterMode::Off, false),
            (FilterMode::Off, true),
            (FilterMode::Nes, false),
            (FilterMode::Nes, true),
        ] {
            let x = render_script(&[(0.0, tone.writes.clone())], 0.5 + N as f64 / FS + 0.01, filter, bl);
            let start = (0.5 * FS) as usize;
            let r = alias_report(&x[start..start + N], tone.f0);
            cells.push(format!("{:.1} dB / {:.1} dBc", r.alias_db, r.worst_dbc));
        }
        println!("| {} | {:.1} | {} |", tone.name, tone.f0, cells.join(" | "));
    }
}

// ---------------------------------------------------------------- tuning

fn tuning() {
    let mut md = String::new();
    md.push_str("# Tuning: MIDI note -> period register, error in cents\n\n");
    md.push_str("Generated by `cargo run --release -p chip --example lab -- tuning`. NTSC CPU clock\n");
    md.push_str("1.789773 MHz, A4 = 440 Hz, nearest period. `-` = outside the register range\n");
    md.push_str("(2A03 pulse periods below 8 are muted by the sweep unit).\n\n");
    md.push_str("| Note | MIDI | Hz | Pulse t | Pulse ¢ | Triangle t | Triangle ¢ | VRC6 pulse t | VRC6 pulse ¢ | Saw t | Saw ¢ |\n");
    md.push_str("|---|---|---|---|---|---|---|---|---|---|---|\n");
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let mut worst = [(0.0f64, 0u8); 4];
    for note in 21..=108u8 {
        let name = format!("{}{}", NAMES[note as usize % 12], note as i32 / 12 - 1);
        let mut row = format!("| {name} | {note} | {:.2} ", pitch::midi_hz(note as f64));
        for (i, osc) in [Osc::Pulse, Osc::Triangle, Osc::Vrc6Pulse, Osc::Saw].into_iter().enumerate() {
            match pitch::period_for(osc, note) {
                Some(t) => {
                    let e = pitch::cents_error(osc, note, t);
                    if (36..=96).contains(&note) && e.abs() > worst[i].0.abs() {
                        worst[i] = (e, note);
                    }
                    row.push_str(&format!("| {t} | {e:+.1} "));
                }
                None => row.push_str("| - | - "),
            }
        }
        row.push_str("|\n");
        md.push_str(&row);
    }
    md.push_str("\nWorst error in C2..C7 (MIDI 36..96): ");
    for (i, n) in ["pulse", "triangle", "VRC6 pulse", "saw"].iter().enumerate() {
        md.push_str(&format!("{n} {:+.1}¢ (MIDI {}); ", worst[i].0, worst[i].1));
    }
    md.push('\n');
    std::fs::create_dir_all("docs").unwrap();
    std::fs::write("docs/tuning.md", &md).unwrap();
    println!("wrote docs/tuning.md");
    println!("{}", md.lines().last().unwrap());
}

// ---------------------------------------------------------------- cost

fn cost() {
    let busy = {
        let mut w = init();
        w.extend([
            (0x4000, 0xBF), (0x4002, 0xFD), (0x4003, 0x08),
            (0x4004, 0x7F), (0x4006, 0x7E), (0x4007, 0x08),
            (0x4008, 0xFF), (0x400A, 0xFD), (0x400B, 0x08),
            (0x400C, 0x3F), (0x400E, 0x04), (0x400F, 0x08),
            (0x4010, 0x4F), (0x4012, 0x00), (0x4013, 0x20), (0x4015, 0x1F),
            (0x9000, 0x3F), (0x9001, 0x80), (0x9002, 0x80),
            (0xA000, 0x5F), (0xA001, 0x40), (0xA002, 0x80),
            (0xB000, 0x2A), (0xB001, 0x50), (0xB002, 0x80),
        ]);
        w
    };
    let mut ultrasonic = init();
    ultrasonic.extend([(0x4008, 0xFF), (0x400A, 0x00), (0x400B, 0x08)]);
    let mut noise0 = init();
    noise0.extend([(0x400C, 0x3F), (0x400E, 0x00), (0x400F, 0x08)]);
    println!("| Case | Events/s | ns per output sample | % of real time |");
    println!("|---|---|---|---|");
    for (name, w) in [
        ("silent", init()),
        ("all 8 channels busy (A4-ish)", busy),
        ("noise, period index 0 (worst noise)", noise0),
        ("triangle t=0, ultrasonic (worst case)", ultrasonic),
    ] {
        let secs = 60.0;
        let mut s = Box::new(Synth::new(FS));
        samples::build(&mut s.rom);
        for (a, v) in w {
            s.write(a, v);
        }
        let mut buf = vec![0f32; 256];
        let t0 = Instant::now();
        let n = (secs * FS) as usize / buf.len();
        for _ in 0..n {
            s.render(&mut buf);
        }
        let el = t0.elapsed().as_secs_f64();
        let samples = (n * buf.len()) as f64;
        println!(
            "| {name} | {:.0} | {:.1} | {:.2}% |",
            s.events as f64 / secs,
            el / samples * 1e9,
            el / secs * 100.0
        );
    }
}

// ---------------------------------------------------------------- listening pack

struct Script {
    ev: Vec<(f64, Writes)>,
}

impl Script {
    fn new() -> Self {
        Self { ev: vec![(0.0, init())] }
    }
    fn at(&mut self, t: f64, w: &[(u16, u8)]) {
        self.ev.push((t, w.to_vec()));
    }
    fn finish(mut self) -> Vec<(f64, Writes)> {
        self.ev.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        self.ev
    }
}

fn pulse_note(note: u8, duty: u8, ctl: u8) -> Writes {
    let t = pitch::period_for(Osc::Pulse, note).unwrap();
    vec![(0x4000, (duty << 6) | ctl), (0x4002, t as u8), (0x4003, 0x08 | (t >> 8) as u8)]
}

fn pack() {
    let dir = Path::new("docs/audio");
    std::fs::create_dir_all(dir).unwrap();
    let mut files: Vec<PackFile> = Vec::new();

    // 1. Pulse: duty sweep (constant volume), then hardware sweep and envelope.
    let mut s = Script::new();
    let mut t = 0.0;
    for note in [57u8, 69] {
        for duty in 0..4 {
            s.at(t, &pulse_note(note, duty, 0x30 | 11));
            t += 0.7;
        }
    }
    s.at(t, &[(0x4000, 0x30)]);
    t += 0.2;
    // Sweep down in pitch (period grows) until the overflow mutes it.
    s.at(t, &pulse_note(69, 2, 0x30 | 11));
    s.at(t, &[(0x4001, 0x80 | 0x30 | 0x02)]);
    t += 1.5;
    // Sweep up in pitch (negate) until t < 8 mutes it.
    s.at(t, &pulse_note(57, 2, 0x30 | 11));
    s.at(t, &[(0x4001, 0x80 | 0x30 | 0x08 | 0x03)]);
    t += 1.5;
    s.at(t, &[(0x4001, 0x08)]);
    // Hardware envelope decay at several rates.
    for v in [2u8, 6, 12] {
        s.at(t, &pulse_note(64, 1, v));
        t += 1.2;
    }
    files.push(("pulse_duty_sweep", s.finish(), t, "pulse 1: duties 12.5/25/50/75% at A3 and A4; hardware sweep down then up (both end in sweep muting); envelope decay at V=2, 6, 12"));

    // 2. Triangle bass line.
    let mut s = Script::new();
    let line = [28u8, 28, 40, 28, 31, 33, 35, 40, 33, 33, 45, 33, 35, 36, 38, 40];
    let mut t = 0.0;
    for (i, &n) in line.iter().enumerate() {
        let p = pitch::period_for(Osc::Triangle, n).unwrap();
        // Staccato on odd steps via a short linear counter (control clear).
        let ctl = if i % 2 == 1 { 20 } else { 0xFF };
        s.at(t, &[(0x4008, ctl), (0x400A, p as u8), (0x400B, 0x08 | (p >> 8) as u8)]);
        t += 0.25;
    }
    s.at(t, &[(0x4008, 0x80)]);
    t += 0.3;
    // Ultrasonic "silencing" (period 0) as some games do: listen for the pop.
    s.at(t, &[(0x4008, 0xFF), (0x400A, 0xFD), (0x400B, 0x08)]);
    t += 0.5;
    s.at(t, &[(0x400A, 0x00), (0x400B, 0x08)]);
    t += 0.7;
    files.push(("triangle_bass", s.finish(), t, "triangle: E1..D2 bass line (odd steps staccato via the linear counter), then a note silenced by writing period 0 (ultrasonic, audible pop)"));

    // 3. Noise: both modes over all 16 periods, high to low.
    let mut s = Script::new();
    let mut t = 0.0;
    for mode in [0u8, 0x80] {
        for idx in 0..16u8 {
            s.at(t, &[(0x400C, 0x30 | 10), (0x400E, mode | idx), (0x400F, 0x08)]);
            t += 0.22;
        }
        s.at(t, &[(0x400C, 0x30)]);
        t += 0.3;
    }
    files.push(("noise_modes", s.finish(), t, "noise: long mode periods 0..15, then short (periodic) mode periods 0..15"));

    // 4. VRC6 saw.
    let mut s = Script::new();
    let mut t = 0.0;
    for (rate, notes) in [(42u8, [33u8, 45, 40, 38]), (20, [33, 45, 40, 38]), (50, [33, 45, 40, 38])] {
        for n in notes {
            let p = pitch::period_for(Osc::Saw, n).unwrap();
            s.at(t, &[(0xB000, rate), (0xB001, p as u8), (0xB002, 0x80 | (p >> 8) as u8)]);
            t += 0.45;
        }
    }
    s.at(t, &[(0xB002, 0x00)]);
    t += 0.3;
    files.push(("vrc6_saw", s.finish(), t, "VRC6 saw: A1 A2 E2 D2 at accumulator rate 42 (full), 20 (quieter), 50 (wraps: distorted)"));

    // 5. VRC6 pulse duties.
    let mut s = Script::new();
    let mut t = 0.0;
    let p = pitch::period_for(Osc::Vrc6Pulse, 57).unwrap();
    for d in 0..8u8 {
        s.at(t, &[(0x9000, (d << 4) | 12), (0x9001, p as u8), (0x9002, 0x80 | (p >> 8) as u8)]);
        t += 0.45;
    }
    s.at(t, &[(0x9002, 0x00)]);
    t += 0.3;
    files.push(("vrc6_pulse_duties", s.finish(), t, "VRC6 pulse 1 at A3: duty 1/16 .. 8/16"));

    // 6. DPCM kit at the native rate and slower rates.
    let mut s = Script::new();
    let mut rom = Box::new([0u8; chip::dmc::ROM_SIZE]);
    let refs = samples::build(&mut rom);
    let mut t = 0.0;
    for rate in [15u8, 12, 9] {
        for r in refs {
            s.at(t, &[(0x4010, rate), (0x4011, samples::START_LEVEL), (0x4012, r.addr_reg), (0x4013, r.len_reg), (0x4015, 0x0F), (0x4015, 0x1F)]);
            t += 0.6;
        }
    }
    files.push(("dpcm_kit", s.finish(), t, "DMC: our synthesised kick, snare, tom, crash at rate 15 (native), 12 and 9"));

    for (name, script, secs, desc) in &files {
        for (suffix, mode) in [("nes", FilterMode::Nes), ("raw", FilterMode::Off)] {
            let x = render_script(script, *secs, mode, true);
            let path = dir.join(format!("{name}_{suffix}.wav"));
            write_wav(&path, &x);
            let peak = x.iter().fold(0f32, |m, v| m.max(v.abs()));
            let rms = (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
            let clipped = x.iter().filter(|v| v.abs() >= 1.0).count();
            println!(
                "{} ({:.1} s, peak {peak:.3}, rms {rms:.3}, clipped {clipped}): {desc}",
                path.display(),
                secs
            );
        }
    }

    // 7. Aliasing A/B: a high arpeggio, naive point sampling vs band-limited.
    let mut s = Script::new();
    let mut t = 0.0;
    for _ in 0..4 {
        for n in [84u8, 88, 91, 96, 100, 103] {
            s.at(t, &pulse_note(n, 0, 0x30 | 10));
            t += 0.12;
        }
    }
    s.at(t, &[(0x4000, 0x30)]);
    t += 0.3;
    let script = s.finish();
    for (suffix, bl) in [("naive", false), ("bandlimited", true)] {
        let x = render_script(&script, t, FilterMode::Nes, bl);
        let path = dir.join(format!("aliasing_{suffix}.wav"));
        write_wav(&path, &x);
        println!("{} ({t:.1} s): 12.5% pulse arpeggio C6..G8, {suffix}", path.display());
    }
}

/// Strongest spectral peak (parabolic interpolation) between 20 Hz and `max_hz`.
fn peak_hz(x: &[f32], max_hz: f64) -> f64 {
    let n = x.len();
    let p = power_spectrum(x);
    let bin = FS / n as f64;
    let hi = ((max_hz / bin) as usize).min(p.len() - 2);
    let lo = (20.0 / bin) as usize;
    let k = (lo..hi).max_by(|&a, &b| p[a].partial_cmp(&p[b]).unwrap()).unwrap();
    let (a, b, c) = (p[k - 1].ln(), p[k].ln(), p[k + 1].ln());
    let d = 0.5 * (a - c) / (a - 2.0 * b + c);
    (k as f64 + d) * bin
}

/// Renders notes through each oscillator and checks the detected pitch.
fn verify() {
    const N: usize = 1 << 16;
    println!("| Osc | MIDI | target Hz | period | formula Hz | measured Hz | measured vs formula | vs target |");
    println!("|---|---|---|---|---|---|---|---|");
    let mut worst: f64 = 0.0;
    for (osc, notes) in [
        (Osc::Pulse, [45u8, 57, 69, 81, 93]),
        (Osc::Triangle, [28, 40, 52, 64, 76]),
        (Osc::Vrc6Pulse, [33, 45, 57, 69, 81]),
        (Osc::Saw, [33, 45, 57, 69, 81]),
    ] {
        for note in notes {
            let t = pitch::period_for(osc, note).unwrap();
            let mut w = init();
            w.extend(match osc {
                Osc::Pulse => pulse_note(note, 2, 0x30 | 12),
                Osc::Triangle => vec![(0x4008, 0xFF), (0x400A, t as u8), (0x400B, 0x08 | (t >> 8) as u8)],
                Osc::Vrc6Pulse => vec![(0x9000, 0x7C), (0x9001, t as u8), (0x9002, 0x80 | (t >> 8) as u8)],
                Osc::Saw => vec![(0xB000, 42), (0xB001, t as u8), (0xB002, 0x80 | (t >> 8) as u8)],
            });
            let x = render_script(&[(0.0, w)], 0.3 + N as f64 / FS, FilterMode::Off, true);
            let s0 = (0.3 * FS) as usize;
            let f = osc.frequency(t);
            let m = peak_hz(&x[s0..s0 + N], f * 1.5);
            let e = 1200.0 * (m / f).log2();
            worst = worst.max(e.abs());
            println!(
                "| {osc:?} | {note} | {:.2} | {t} | {f:.2} | {m:.2} | {e:+.2}¢ | {:+.1}¢ |",
                pitch::midi_hz(note as f64),
                1200.0 * (m / pitch::midi_hz(note as f64)).log2()
            );
        }
    }
    println!("
worst |measured - formula| = {worst:.2} cents");
    assert!(worst < 1.0, "pitch mismatch");
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("alias") => alias(),
        Some("tuning") => tuning(),
        Some("cost") => cost(),
        Some("pack") => pack(),
        Some("verify") => verify(),
        _ => eprintln!("usage: lab alias|tuning|cost|pack"),
    }
}
