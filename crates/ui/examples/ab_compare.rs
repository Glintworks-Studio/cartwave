//! Batch 4c §4: the A/B measurements (internal tool; docs/ab/AB.md).
//!
//!   cargo run --release -p ui --example ab_compare -- <name> <segs|-> <ours.wav> <ref.wav>... [--pack <dir>] [--tsv <file>]
//!
//! For each reference render: the lag (cross-correlation over both
//! polarities), the level (least-squares gain), the waveform error after
//! alignment and gain, the spectral difference per octave band, the DC; and,
//! per segment of a test pattern (`ab patterns` writes the `.seg` files), the
//! measurement its kind calls for: pitch (cents), envelope and length timing
//! (frames), level (dB), noise spectra, DPCM step heights. Rows are TSV:
//! name, segment, metric, then one value per reference. `--pack` writes the
//! aligned, level-matched WAVs for listening (A = ours, B, C = references).

use std::f64::consts::PI;
use std::fmt::Write as _;
use std::path::Path;

const SR: f64 = 48_000.0;
const CPU_HZ: f64 = 1_789_772.727_272_7;
const FRAME_S: f64 = 29_780.5 / CPU_HZ;

fn read_wav(p: &str) -> Vec<f64> {
    let b = std::fs::read(p).unwrap_or_else(|e| panic!("{p}: {e}"));
    assert_eq!(&b[0..4], b"RIFF", "{p}: not a WAV");
    let mut i = 12;
    let (mut fmt, mut ch, mut bits, mut sr) = (1u16, 1u16, 16u16, 48_000u32);
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let n = u32::from_le_bytes(b[i + 4..i + 8].try_into().unwrap()) as usize;
        let d = &b[i + 8..(i + 8 + n).min(b.len())];
        if id == b"fmt " {
            fmt = u16::from_le_bytes([d[0], d[1]]);
            ch = u16::from_le_bytes([d[2], d[3]]);
            sr = u32::from_le_bytes(d[4..8].try_into().unwrap());
            bits = u16::from_le_bytes([d[14], d[15]]);
            if fmt == 0xFFFE {
                fmt = u16::from_le_bytes([d[24], d[25]]);
            }
        } else if id == b"data" {
            assert_eq!(sr, 48_000, "{p}: expected 48 kHz, got {sr}");
            let bps = bits as usize / 8;
            let frames = d.len() / (bps * ch as usize);
            return (0..frames)
                .map(|f| {
                    let mut s = 0.0;
                    for c in 0..ch as usize {
                        let o = (f * ch as usize + c) * bps;
                        s += match (fmt, bits) {
                            (3, 32) => f32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as f64,
                            (1, 16) => i16::from_le_bytes([d[o], d[o + 1]]) as f64 / 32768.0,
                            (1, 24) => (i32::from_le_bytes([0, d[o], d[o + 1], d[o + 2]]) >> 8) as f64 / 8_388_608.0,
                            (1, 32) => i32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as f64 / 2_147_483_648.0,
                            _ => panic!("{p}: format {fmt}/{bits} unsupported"),
                        };
                    }
                    s / ch as f64
                })
                .collect();
        }
        i += 8 + n + (n & 1);
    }
    panic!("{p}: no data chunk")
}

fn write_wav16(p: &Path, x: &[f64]) {
    let n = (x.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + n as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + n).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&48_000u32.to_le_bytes());
    b.extend_from_slice(&96_000u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&n.to_le_bytes());
    for v in x {
        b.extend_from_slice(&((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    std::fs::write(p, b).unwrap();
}

// ------------------------------------------------------------------ FFT

fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
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
        let a = 2.0 * PI / len as f64 * if inverse { 1.0 } else { -1.0 };
        let (wr, wi) = (a.cos(), a.sin());
        for s in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (ur, ui) = (re[s + k], im[s + k]);
                let (xr, xi) = (re[s + k + len / 2], im[s + k + len / 2]);
                let (vr, vi) = (xr * cr - xi * ci, xr * ci + xi * cr);
                re[s + k] = ur + vr;
                im[s + k] = ui + vi;
                re[s + k + len / 2] = ur - vr;
                im[s + k + len / 2] = ui - vi;
                let t = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = t;
            }
        }
        len <<= 1;
    }
    if inverse {
        for v in re.iter_mut().chain(im.iter_mut()) {
            *v /= n as f64;
        }
    }
}

/// Lag (ref index = ours index + lag) and sign maximising the cross-correlation.
fn align(ours: &[f64], r: &[f64], max_lag: usize) -> (i64, f64) {
    let len = (ours.len().max(r.len()) + max_lag).min(1 << 21);
    let n = (2 * len).next_power_of_two();
    let mut ar = vec![0.0; n];
    let mut ai = vec![0.0; n];
    let mut br = vec![0.0; n];
    let mut bi = vec![0.0; n];
    for (i, v) in ours.iter().take(len).enumerate() {
        ar[i] = *v;
    }
    for (i, v) in r.iter().take(len).enumerate() {
        br[i] = *v;
    }
    fft(&mut ar, &mut ai, false);
    fft(&mut br, &mut bi, false);
    // conj(A) * B: correlation of ref against ours.
    let mut cr = vec![0.0; n];
    let mut ci = vec![0.0; n];
    for k in 0..n {
        cr[k] = ar[k] * br[k] + ai[k] * bi[k];
        ci[k] = ar[k] * bi[k] - ai[k] * br[k];
    }
    fft(&mut cr, &mut ci, true);
    // Coarse first, on the 1 ms RMS envelopes (a periodic tone correlates
    // equally well a period or a frame away; the envelope's structure doesn't),
    // then the sample lag within 2 ms of it.
    // The envelope of the first difference: DC-free (a reference without a
    // high-pass keeps DC, which would dominate) and weighted to the edges.
    let env = |x: &[f64]| -> Vec<f64> {
        let d: Vec<f64> = x.windows(2).map(|w| w[1] - w[0]).collect();
        let e: Vec<f64> = d.chunks(48).map(rms).collect();
        let m = e.iter().sum::<f64>() / e.len().max(1) as f64;
        e.iter().map(|v| v - m).collect()
    };
    let (eo, er) = (env(ours), env(r));
    let hops = (max_lag / 48) as i64;
    let mut coarse = (0i64, f64::MIN);
    for l in -hops..=hops {
        let mut c = 0.0;
        for (i, v) in eo.iter().enumerate() {
            let j = i as i64 + l;
            if j >= 0 && (j as usize) < er.len() {
                c += v * er[j as usize];
            }
        }
        if c > coarse.1 {
            coarse = (l, c);
        }
    }
    let (lo, hi) = (coarse.0 * 48 - 96, coarse.0 * 48 + 96);
    let mut best = (0i64, 0.0f64);
    for lag in lo.max(-(max_lag as i64))..=hi.min(max_lag as i64) {
        let v = cr[lag.rem_euclid(n as i64) as usize];
        if v.abs() > best.1.abs() {
            best = (lag, v);
        }
    }
    (best.0, best.1.signum())
}

/// RMS with the mean removed: the level the ear compares (a reference
/// without a high-pass keeps a pulse's DC, which isn't heard).
fn ac_rms(x: &[f64]) -> f64 {
    let m = x.iter().sum::<f64>() / x.len().max(1) as f64;
    (x.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

/// The waveform error of `r` against `o` (dB re `o`): the best lag within
/// ±2 ms of `lag` (this stretch's own alignment: the NSF player's writes land
/// a few hundred cycles from ours, so each note's phase differs slightly),
/// then the least-squares gain; means removed.
fn local_error(o: &[f64], a0: usize, r: &[f64], lag: i64, sign: f64) -> f64 {
    let m = o.iter().sum::<f64>() / o.len().max(1) as f64;
    let at = |k: usize, l: i64| -> f64 {
        let j = (a0 + k) as i64 + l;
        if j >= 0 && (j as usize) < r.len() { sign * r[j as usize] } else { 0.0 }
    };
    let mut best = (lag, 0.0f64);
    for l in lag - 96..=lag + 96 {
        let c: f64 = (0..o.len()).step_by(2).map(|k| (o[k] - m) * at(k, l)).sum();
        if c.abs() > best.1.abs() {
            best = (l, c);
        }
    }
    // This stretch's own polarity (noise has none to speak of globally).
    let ps = best.1.signum();
    let rr: Vec<f64> = (0..o.len()).map(|k| ps * at(k, best.0)).collect();
    let rm = rr.iter().sum::<f64>() / rr.len().max(1) as f64;
    let (sxy, syy) = o.iter().zip(&rr).fold((0.0, 0.0), |(a, b), (x, y)| (a + (x - m) * (y - rm), b + (y - rm) * (y - rm)));
    let g = sxy / syy.max(1e-30);
    let e: f64 = o.iter().zip(&rr).map(|(x, y)| ((x - m) - g * (y - rm)).powi(2)).sum();
    let p: f64 = o.iter().map(|x| (x - m) * (x - m)).sum();
    10.0 * (e / p.max(1e-30)).max(1e-12).log10()
}

/// One-pole high-pass at 20 Hz (the same on every signal).
fn hp(x: &[f64]) -> Vec<f64> {
    let rc = 1.0 / (2.0 * PI * 20.0);
    let a = rc / (rc + 1.0 / SR);
    let (mut y, mut px) = (0.0, x.first().copied().unwrap_or(0.0));
    x.iter()
        .map(|&v| {
            y = a * (y + v - px);
            px = v;
            y
        })
        .collect()
}

fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len().max(1) as f64).sqrt()
}
fn db(x: f64) -> f64 {
    20.0 * x.max(1e-12).log10()
}

/// Welch power spectrum (8192-point Hann, half overlap): power per bin.
fn psd(x: &[f64]) -> Vec<f64> {
    const N: usize = 8192;
    let w: Vec<f64> = (0..N).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / N as f64).cos()).collect();
    let mut acc = vec![0.0; N / 2];
    let mut k = 0;
    let mut s = 0;
    while s + N <= x.len() {
        let mut re: Vec<f64> = (0..N).map(|i| x[s + i] * w[i]).collect();
        let mut im = vec![0.0; N];
        fft(&mut re, &mut im, false);
        for b in 0..N / 2 {
            acc[b] += re[b] * re[b] + im[b] * im[b];
        }
        k += 1;
        s += N / 2;
    }
    for v in &mut acc {
        *v /= k.max(1) as f64;
    }
    acc
}

const BANDS: [f64; 10] = [31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];

fn band_power(p: &[f64], centre: f64) -> f64 {
    let bin = SR / 8192.0;
    let (lo, hi) = (centre / 2f64.sqrt(), centre * 2f64.sqrt());
    p.iter().enumerate().filter(|(i, _)| { let f = *i as f64 * bin; f >= lo && f < hi }).map(|(_, v)| v).sum()
}

/// The strongest frequency above 20 Hz (zero-padded FFT, parabolic peak).
fn pitch(x: &[f64]) -> f64 {
    let n = (x.len() * 4).next_power_of_two().max(1 << 16);
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    let m = x.len();
    for i in 0..m {
        re[i] = x[i] * (0.5 - 0.5 * (2.0 * PI * i as f64 / m as f64).cos());
    }
    fft(&mut re, &mut im, false);
    let mag: Vec<f64> = (0..n / 2).map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt().max(1e-20).ln()).collect();
    let lo = (20.0 * n as f64 / SR) as usize;
    let k = (lo..n / 2 - 1).max_by(|a, b| mag[*a].total_cmp(&mag[*b])).unwrap();
    let (a, b, c) = (mag[k - 1], mag[k], mag[k + 1]);
    let d = 0.5 * (a - c) / (a - 2.0 * b + c);
    (k as f64 + d) * SR / n as f64
}

fn centroid(x: &[f64]) -> f64 {
    let m = x.iter().sum::<f64>() / x.len().max(1) as f64;
    let p = psd(&x.iter().map(|v| v - m).collect::<Vec<_>>());
    let bin = SR / 8192.0;
    let (num, den) = p.iter().enumerate().fold((0.0, 0.0), |(n, d), (i, v)| (n + i as f64 * bin * v, d + v));
    num / den.max(1e-30)
}

/// RMS envelope in 1 ms hops.
fn envelope(x: &[f64]) -> Vec<f64> {
    let h = 48;
    x.chunks(h).map(rms).collect()
}

/// When (s) the envelope first falls `drop_db` below its peak (after the peak).
fn fall_time(env: &[f64], drop_db: f64) -> Option<f64> {
    let (pk, pi) = env.iter().take(60).enumerate().fold((0.0, 0), |(m, mi), (i, v)| if *v > m { (*v, i) } else { (m, mi) });
    let th = pk * 10f64.powf(-drop_db / 20.0);
    env.iter().enumerate().skip(pi).find(|(_, v)| **v < th).map(|(i, _)| i as f64 / 1000.0)
}

struct Seg {
    at: usize,
    n: usize,
    kind: String,
    label: String,
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let name = a[0].clone();
    let segs: Vec<Seg> = if a[1] == "-" {
        Vec::new()
    } else {
        std::fs::read_to_string(&a[1])
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                Seg { at: f[0].parse().unwrap(), n: f[1].parse().unwrap(), kind: f[2].into(), label: f[3].into() }
            })
            .collect()
    };
    let mut files = Vec::new();
    let mut pack: Option<String> = None;
    let mut tsv: Option<String> = None;
    let mut i = 2;
    while i < a.len() {
        match a[i].as_str() {
            "--pack" => {
                pack = Some(a[i + 1].clone());
                i += 2;
            }
            "--tsv" => {
                tsv = Some(a[i + 1].clone());
                i += 2;
            }
            _ => {
                files.push(a[i].clone());
                i += 1;
            }
        }
    }
    let raw_ours = read_wav(&files[0]);
    let raw_refs: Vec<Vec<f64>> = files[1..].iter().map(|f| read_wav(f)).collect();
    // Levels, timing and noise are measured after the same 20 Hz high-pass on
    // all three (a reference without its high-pass keeps DC, which would
    // otherwise dominate them); the bands, the DC and the DMC steps use the
    // raw signals (they are the filter response being compared).
    let ours = hp(&raw_ours);
    let refs: Vec<Vec<f64>> = raw_refs.iter().map(|r| hp(r)).collect();
    let mut rows: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut row = |seg: &str, metric: &str, vals: Vec<String>| rows.push((seg.to_string(), metric.to_string(), vals));
    // Align, sign and gain per reference: (lag, sign, gain, aligned, aligned raw).
    let mut fit = Vec::new();
    for (r, raw) in refs.iter().zip(&raw_refs) {
        // ±3 s: gme skips a track's leading silence, so its lag depends on
        // the content; NSFPlay's is a constant ≈ 120 ms.
        let (lag, sign) = align(&ours, r, 144_000);
        let at = |x: &[f64], k: usize| -> Option<f64> {
            let j = k as i64 + lag;
            (j >= 0 && (j as usize) < x.len()).then(|| sign * x[j as usize])
        };
        // The level match: AC RMS over the overlap (a global least-squares
        // gain is biased low by the small phase differences per note).
        let (mut oo, mut rr) = (Vec::new(), Vec::new());
        for k in 0..ours.len() {
            if let Some(y) = at(r, k) {
                oo.push(ours[k]);
                rr.push(y);
            }
        }
        let g = ac_rms(&oo) / ac_rms(&rr).max(1e-30);
        let aligned: Vec<f64> = (0..ours.len()).map(|k| at(r, k).unwrap_or(0.0) * g).collect();
        let aligned_raw: Vec<f64> = (0..ours.len()).map(|k| at(raw, k).unwrap_or(0.0) * g).collect();
        fit.push((lag, sign, g, aligned, aligned_raw));
    }
    let f3 = |v: f64| format!("{v:.3}");
    let f1 = |v: f64| format!("{v:.1}");
    let f2 = |v: f64| format!("{v:.2}");
    row("all", "lag_ms", fit.iter().map(|f| f3(f.0 as f64 / 48.0)).collect());
    // How much of our render each reference covers (a player that stops on
    // silence covers less; everything below is measured on what it rendered).
    row("all", "coverage_%", fit.iter().zip(&refs).map(|(f, r)| f1(100.0 * ((r.len() as i64 - f.0).clamp(0, ours.len() as i64) as f64 / ours.len() as f64))).collect());
    row("all", "polarity", fit.iter().map(|f| if f.1 > 0.0 { "same".into() } else { "inverted".into() }).collect());
    row("all", "level_db (ref vs ours, before matching)", fit.iter().map(|f| f1(-db(f.2))).collect());
    // The waveform error per stretch (each segment of a pattern, or each
    // second of a song), each with its own fine alignment; the median.
    let stretches: Vec<(usize, usize)> = if segs.is_empty() {
        (0..ours.len() / 48_000).map(|i| (i * 48_000, (i + 1) * 48_000)).collect()
    } else {
        segs.iter()
            .filter(|s| s.kind != "gap")
            .map(|s| (((s.at as f64 + 3.0) * FRAME_S * SR) as usize, (((s.at + s.n) as f64 - 2.0) * FRAME_S * SR) as usize))
            .filter(|(a, b)| b > a && *b <= ours.len())
            .collect()
    };
    let errs: Vec<Vec<f64>> = fit
        .iter()
        .zip(&refs)
        .map(|(f, r)| {
            let mut e: Vec<f64> = stretches
                .iter()
                .filter(|(a, b)| ac_rms(&ours[*a..*b]) > 1e-4 && ((*b as i64 + f.0) as usize) <= r.len())
                .map(|(a, b)| local_error(&ours[*a..*b], *a, r, f.0, f.1))
                .collect();
            e.sort_by(|a, b| a.total_cmp(b));
            e
        })
        .collect();
    row("all", "waveform_error_db median (per stretch, aligned, gain-matched)", errs.iter().map(|e| e.get(e.len() / 2).map_or("-".into(), |v| f1(*v))).collect());
    row("all", "waveform_error_db 90th percentile", errs.iter().map(|e| e.get(e.len() * 9 / 10).map_or("-".into(), |v| f1(*v))).collect());
    let dc_o = raw_ours.iter().sum::<f64>() / raw_ours.len() as f64;
    row("all", "dc_raw (ours, fraction of full scale)", vec![format!("{dc_o:.5}")]);
    row("all", "dc_raw (ref, level-matched)", fit.iter().map(|f| format!("{:.5}", f.4.iter().sum::<f64>() / f.4.len() as f64)).collect());
    let po = psd(&raw_ours);
    for &c in &BANDS {
        let bo = band_power(&po, c);
        row("all", &format!("band_{c}Hz_db (raw)"), fit.iter().map(|f| f1(10.0 * (band_power(&psd(&f.4), c) / bo.max(1e-30)).log10())).collect());
    }
    // Per segment. Levels are relative to the file's first measured segment
    // (its anchor), so a file-wide gain doesn't enter them.
    let span = |s: &Seg| {
        (
            ((s.at as f64 + 3.0) * FRAME_S * SR) as usize,
            (((s.at + s.n) as f64 - 2.0) * FRAME_S * SR) as usize,
            (s.at as f64 * FRAME_S * SR) as usize,
            ((s.at + s.n) as f64 * FRAME_S * SR) as usize,
        )
    };
    let leveled = |k: &str| matches!(k, "tone" | "level" | "dmc" | "noise");
    let anchor = segs.iter().find(|s| leveled(&s.kind) && { let (a0, a1, _, _) = span(s); a1 > a0 && a1 <= ours.len() && ac_rms(&ours[a0..a1]) > 1e-3 });
    let anchor_db = anchor.map(|s| {
        let (a0, a1, _, _) = span(s);
        (db(ac_rms(&ours[a0..a1])), fit.iter().map(|f| db(ac_rms(&f.3[a0..a1]))).collect::<Vec<f64>>())
    });
    let first_step = segs.iter().find(|s| s.kind == "step").map(|s| span(s).2);
    let step_h = |x: &[f64], k: usize| -> f64 {
        let after: f64 = x[k + 10..k + 72].iter().sum::<f64>() / 62.0;
        let before: f64 = x[k - 72..k - 10].iter().sum::<f64>() / 62.0;
        after - before
    };
    for s in &segs {
        let (a0, a1, full0, full1) = span(s);
        if a1 <= a0 || a1 > ours.len() {
            continue;
        }
        let o = &ours[a0..a1];
        let rs: Vec<&[f64]> = fit.iter().map(|f| &f.3[a0..a1]).collect();
        // A reference that stopped before this segment has zeros here: its
        // values would be meaningless, so the row says so and they are skipped.
        let covered: Vec<bool> = fit.iter().zip(&refs).map(|(f, r)| (full1 as i64 + f.0) <= r.len() as i64).collect();
        let lab = s.label.as_str();
        if covered.iter().any(|c| !c) {
            row(lab, "not covered by", covered.iter().map(|c| if *c { "".into() } else { "stopped".into() }).collect());
        }
        let cv = |j: usize, v: String| if covered[j] { v } else { "-".into() };
        if s.kind != "gap" && ac_rms(o) > 1e-4 {
            row(lab, "waveform_error_db", fit.iter().zip(&refs).enumerate().map(|(j, (f, r))| cv(j, f1(local_error(o, a0, r, f.0, f.1)))).collect());
        }
        if leveled(&s.kind) && ac_rms(o) > 1e-4 {
            if let (Some(a), Some((ao, ar))) = (anchor, &anchor_db) {
                row(
                    lab,
                    &format!("level_rel_db (vs \"{}\"; ours {:+.1} dB)", a.label, db(ac_rms(o)) - ao),
                    rs.iter().enumerate().map(|(j, r)| cv(j, f1((db(ac_rms(r)) - ar[j]) - (db(ac_rms(o)) - ao)))).collect(),
                );
            }
        }
        match s.kind.as_str() {
            "tone" => {
                let po_ = pitch(o);
                row(lab, &format!("pitch_hz ours {po_:.2}; ref cents"), rs.iter().enumerate().map(|(j, r)| cv(j, f2(1200.0 * (pitch(r) / po_).log2()))).collect());
            }
            "sweep" => {
                // The pitch along the sweep: 8-frame windows every 10 frames,
                // while ours still sounds (a sweep can mute the channel).
                let mut track_o = Vec::new();
                let mut track_r: Vec<Vec<String>> = vec![Vec::new(); fit.len()];
                let mut k = 2;
                while k + 8 <= s.n {
                    let w0 = ((s.at + k) as f64 * FRAME_S * SR) as usize;
                    let w1 = ((s.at + k + 8) as f64 * FRAME_S * SR) as usize;
                    if w1 <= ours.len() && ac_rms(&ours[w0..w1]) > 1e-3 {
                        let pw = pitch(&ours[w0..w1]);
                        track_o.push(format!("{pw:.0}"));
                        for (j, f) in fit.iter().enumerate() {
                            let rw = &f.3[w0..w1];
                            track_r[j].push(if ac_rms(rw) > 1e-3 { format!("{:+.0}", 1200.0 * (pitch(rw) / pw).log2()) } else { "silent".into() });
                        }
                    }
                    k += 10;
                }
                row(lab, &format!("pitch track ours Hz [{}]; ref cents", track_o.join(" ")), track_r.iter().enumerate().map(|(j, t)| cv(j, format!("[{}]", t.join(" ")))).collect());
                // How long ours and each reference sound in the segment (frames).
                let sounding = |x: &[f64]| x.chunks((FRAME_S * SR) as usize).filter(|c| ac_rms(c) > 1e-3).count();
                row(lab, &format!("sounding_frames ours {}", sounding(&ours[full0..full1])), fit.iter().enumerate().map(|(j, f)| cv(j, sounding(&f.3[full0..full1]).to_string())).collect());
            }
            "decay" | "length" => {
                let oe = envelope(&ours[full0..full1.min(ours.len())]);
                for d in [6.0, 12.0, 24.0] {
                    let to = fall_time(&oe, d);
                    row(
                        lab,
                        &format!("fall_{d}db_frames ours {}; ref diff", to.map_or("-".into(), |t| format!("{:.2}", t / FRAME_S))),
                        fit.iter()
                            .enumerate()
                            .map(|(j, f)| {
                                let re = envelope(&f.3[full0..full1.min(f.3.len())]);
                                cv(j, match (fall_time(&re, d), to) {
                                    (Some(r), Some(o)) => format!("{:+.2}", (r - o) / FRAME_S),
                                    _ => "-".into(),
                                })
                            })
                            .collect(),
                    );
                }
            }
            "noise" => {
                let co = centroid(o);
                row(lab, &format!("centroid_hz ours {co:.0}; ref ratio %"), rs.iter().enumerate().map(|(j, r)| cv(j, f1(100.0 * (centroid(r) / co - 1.0)))).collect());
                // The LFSR's loop: a render that repeats every 93 (short mode)
                // or 32767 (long) steps of the noise timer correlates with
                // itself at that lag (NTSC timer periods, the wiki's table).
                const NOISE_PERIOD: [f64; 16] = [4.0, 8.0, 16.0, 32.0, 64.0, 96.0, 128.0, 160.0, 202.0, 254.0, 380.0, 508.0, 762.0, 1016.0, 2034.0, 4068.0];
                if let Some(pi) = lab.split("period ").nth(1).and_then(|v| v.trim().parse::<usize>().ok()) {
                    let loops: &[f64] = if lab.contains("mode 1") { &[93.0, 31.0] } else { &[32767.0] };
                    for &steps in loops {
                        let lag = steps * NOISE_PERIOD[pi] / CPU_HZ * SR;
                        if lag + 10.0 < o.len() as f64 / 2.0 {
                            let corr = |x: &[f64]| -> f64 {
                                let m = x.iter().sum::<f64>() / x.len() as f64;
                                let e: f64 = x.iter().map(|v| (v - m) * (v - m)).sum();
                                let l0 = lag.round() as usize;
                                (l0.saturating_sub(2)..=l0 + 2).map(|l| (0..x.len() - l).map(|i| (x[i] - m) * (x[i + l] - m)).sum::<f64>() / e.max(1e-30)).fold(f64::MIN, f64::max)
                            };
                            row(lab, &format!("loop_corr at {steps} steps ({:.1} ms) ours {:.2}", lag / 48.0, corr(o)), rs.iter().enumerate().map(|(j, r)| cv(j, f2(corr(r)))).collect());
                        }
                    }
                }
            }
            "step" => {
                // The jump across the write, relative to the first step's
                // (raw: the DMC's level is a DC offset).
                if let Some(k0) = first_step {
                    let ho = step_h(&raw_ours, full0) / step_h(&raw_ours, k0);
                    row(lab, &format!("step_rel ours {ho:.3}; ref ratio dB"), fit.iter().enumerate().map(|(j, f)| cv(j, f1(db((step_h(&f.4, full0) / step_h(&f.4, k0) / ho).abs())))).collect());
                }
            }
            _ => {}
        }
    }
    let mut out = String::new();
    for (seg, metric, vals) in &rows {
        let _ = writeln!(out, "{name}\t{seg}\t{metric}\t{}", vals.join("\t"));
    }
    print!("{out}");
    if let Some(t) = tsv {
        let mut prev = std::fs::read_to_string(&t).unwrap_or_default();
        prev.push_str(&out);
        std::fs::write(&t, prev).unwrap();
    }
    if let Some(dir) = pack {
        std::fs::create_dir_all(&dir).unwrap();
        // One scale for all three so they stay level-matched, peak at -1 dBFS.
        let peak = std::iter::once(&raw_ours).chain(fit.iter().map(|f| &f.4)).flat_map(|x| x.iter()).fold(0f64, |m, v| m.max(v.abs()));
        let k = 0.891 / peak.max(1e-9);
        let sc = |x: &[f64]| x.iter().map(|v| v * k).collect::<Vec<_>>();
        write_wav16(&Path::new(&dir).join(format!("{name}_A_cartwave.wav")), &sc(&raw_ours));
        for (j, f) in fit.iter().enumerate() {
            let tag = Path::new(&files[1 + j]).file_stem().unwrap().to_string_lossy().to_string();
            let who = if tag.contains("nsfplay") { "nsfplay" } else if tag.contains("gme") { "gme" } else { "ref" };
            write_wav16(&Path::new(&dir).join(format!("{name}_{}_{who}.wav", ["B", "C", "D"][j])), &sc(&f.4));
        }
    }
}
