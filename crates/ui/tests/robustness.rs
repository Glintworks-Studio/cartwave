//! Robustness of the three parsers (`.mid`, `.wav`, `.chipdaw`) against
//! damaged and hostile input (STABILIZE_PROMPT.md §A3).
//!
//! Thousands of seeded inputs per parser: valid seed files of every kind the
//! importers claim to read, mutated (bit flips, interesting values, cut,
//! duplicated and deleted spans, extreme numbers in the JSON), plus pure
//! noise. Each input must give `Ok` or `Err` within a time budget: never a
//! panic, never a hang. Inputs that parse are taken further, through the same
//! steps the app runs (arrange, convert, snapshot, a short render), so a
//! value that slips through the parser can't crash the engine later.
//!
//! Deterministic: the same seeds every run. `CHIPDAW_FUZZ_N` sets the count
//! per parser (default 2000). Run it in debug builds too: they check integer
//! overflow, release builds don't.

use std::sync::mpsc;
use std::time::Duration;

mod common;
use common::*;
use ui::{dpcm, export, file, midi, mock};

// ───────────────────────────── harness ─────────────────────────────

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn chance(&mut self, p: f64) -> bool {
        (self.next() % 1_000_000) as f64 / 1_000_000.0 < p
    }
}

fn count() -> usize {
    std::env::var("CHIPDAW_FUZZ_N").ok().and_then(|v| v.parse().ok()).unwrap_or(2000)
}

/// Runs `f` on its own thread with a time budget. Returns a failure message
/// for a panic or a hang.
fn guarded(budget: Duration, f: impl FnOnce() + Send + 'static) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || {
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let _ = tx.send(r.err().map(|e| {
                e.downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "panic".into())
            }));
        })
        .unwrap();
    match rx.recv_timeout(budget) {
        Ok(None) => {
            let _ = h.join();
            None
        }
        Ok(Some(msg)) => Some(format!("panic: {msg}")),
        Err(_) => Some(format!("hang: no result within {budget:?}")),
    }
}

const INTERESTING: [u8; 10] = [0x00, 0xFF, 0x7F, 0x80, 0x01, 0xFE, 0x40, 0x90, 0xF0, 0x2F];

/// Byte-level mutation: 1..8 random edits.
fn mutate(seed: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut b = seed.to_vec();
    for _ in 0..1 + rng.below(8) {
        if b.is_empty() {
            b.push(rng.next() as u8);
            continue;
        }
        let i = rng.below(b.len());
        match rng.below(9) {
            0 => b[i] ^= 1 << rng.below(8),
            1 => b[i] = INTERESTING[rng.below(INTERESTING.len())],
            2 => b[i] = rng.next() as u8,
            3 => b.truncate(i),
            4 => {
                // Delete a span.
                let n = rng.below(64).min(b.len() - i);
                b.drain(i..i + n);
            }
            5 => {
                // Duplicate a span.
                let n = rng.below(64).min(b.len() - i);
                let span: Vec<u8> = b[i..i + n].to_vec();
                let at = rng.below(b.len());
                b.splice(at..at, span);
            }
            6 => {
                // Overwrite 4 bytes with an extreme 32-bit value (lengths, rates).
                let v: u32 = [0, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF, 0x0FFF_FFFF][rng.below(6)];
                for (k, x) in v.to_le_bytes().iter().enumerate() {
                    if let Some(y) = b.get_mut(i + k) {
                        *y = *x;
                    }
                }
            }
            7 => b.insert(i, rng.next() as u8),
            _ => {
                let v = rng.next().to_be_bytes();
                for (k, x) in v.iter().take(1 + rng.below(4)).enumerate() {
                    if let Some(y) = b.get_mut(i + k) {
                        *y = *x;
                    }
                }
            }
        }
    }
    b
}

fn noise(rng: &mut Rng, prefix: &[u8]) -> Vec<u8> {
    let mut b = prefix.to_vec();
    for _ in 0..rng.below(4096) {
        b.push(rng.next() as u8);
    }
    b
}

/// Runs `n` inputs (mutated seeds, sometimes plain noise) through `check`.
fn campaign(name: &str, seeds: &[Vec<u8>], magic: &[u8], seed: u64, check: fn(Vec<u8>)) {
    let mut rng = Rng(seed);
    let mut failures = Vec::new();
    let n = count();
    // The unmodified seeds must all pass too.
    let inputs = seeds.iter().cloned().map(|s| (usize::MAX, s));
    let mutated = (0..n).map(|i| {
        let input = if rng.chance(0.05) {
            let prefix: &[u8] = if rng.chance(0.5) { magic } else { b"" };
            noise(&mut rng, prefix)
        } else {
            let s = &seeds[rng.below(seeds.len())];
            mutate(s, &mut rng)
        };
        (i, input)
    });
    for (i, input) in inputs.chain(mutated.collect::<Vec<_>>()) {
        let copy = input.clone();
        if let Some(msg) = guarded(Duration::from_secs(20), move || check(copy)) {
            failures.push(format!("{name} input #{i} ({} bytes): {msg}", input.len()));
            if std::env::var("CHIPDAW_FUZZ_DUMP").is_ok() {
                let _ = std::fs::write(format!("fuzz_{name}_{i}.bin"), &input);
            }
            if failures.len() >= 20 {
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

// ───────────────────────────── MIDI ─────────────────────────────







fn midi_seeds() -> Vec<Vec<u8>> {
    let scale: Vec<(u32, u32, u8)> = (0..16).map(|i| (0, 240, 60 + (i % 12) as u8)).collect();
    let mut seeds = vec![
        // Format 0: everything in one track, several channels, a tempo.
        smf(0, 480, &[trk(&[vec![(0, tempo(140.0))], melody(0, &scale), melody(1, &scale)].concat())]),
        // Format 1 with a tempo map that changes.
        smf(1, 96, &[trk(&[(0, tempo(120.0)), (384, tempo(90.0)), (384, tempo(200.0))]), trk(&melody(0, &scale))]),
    ];
    // Running status: one status byte for the whole run, velocity 0 = off.
    let mut rs = vec![0, 0x90, 60, 90];
    for i in 0..32u8 {
        rs.extend([0x60, 60 + i % 12, 0, 0, 61 + i % 12, 90]);
    }
    rs.extend([0x60, 72, 0, 0, 0xFF, 0x2F, 0]);
    seeds.push(smf(1, 480, &[rs]));
    // Many tracks.
    seeds.push(smf(1, 480, &(0..24).map(|t| trk(&[vec![(0, name(&format!("T{t}")))], melody((t % 15) as u8, &scale)].concat())).collect::<Vec<_>>()));
    // Overlapping notes and chords.
    let chords = trk(&[
        (0, vec![0x90, 60, 100]), (0, vec![0x90, 64, 100]), (0, vec![0x90, 67, 100]),
        (120, vec![0x90, 72, 100]), (240, vec![0x80, 60, 0]), (0, vec![0x80, 64, 0]),
        (100, vec![0x80, 72, 0]), (0, vec![0x80, 67, 0]),
    ]);
    seeds.push(smf(1, 480, &[chords]));
    // Notes out of every channel's range, zero-length notes, unclosed notes.
    seeds.push(smf(1, 480, &[trk(&melody(0, &[(0, 240, 0), (0, 240, 5), (0, 240, 127), (0, 0, 60), (0, 0, 62)]))]));
    seeds.push(smf(1, 480, &[trk(&[(0, vec![0x90, 60, 100]), (960, vec![0x90, 64, 100])])]));
    // Drums on channel 10, and a DPCM track.
    seeds.push(smf(1, 480, &[trk(&[vec![(0, name("Drums"))], melody(9, &[(0, 60, 36), (180, 60, 38), (180, 30, 42), (90, 60, 49), (0, 60, 81)])].concat())]));
    seeds.push(smf(1, 480, &[trk(&[vec![(0, name("DPCM"))], melody(3, &[(0, 120, 36), (0, 120, 40), (0, 120, 43), (0, 120, 90)])].concat())]));
    // Sysex, other channel events, an unknown chunk and a long delta.
    let mut misc = smf(1, 480, &[trk(&[
        (0, vec![0xF0, 3, 1, 2, 0xF7]), (0, vec![0xB0, 7, 100]), (0, vec![0xC0, 5]),
        (0, vec![0xE0, 0, 64]), (0x0FFF_FFFF, vec![0x90, 60, 1]), (10, vec![0x80, 60, 0]),
    ])]);
    misc.extend(b"XFIH");
    misc.extend(4u32.to_be_bytes());
    misc.extend([1, 2, 3, 4]);
    seeds.push(misc);
    seeds
}

fn check_midi(b: Vec<u8>) {
    if let Ok(song) = midi::parse(&b) {
        for vrc6 in [false, true] {
            let imp = midi::arrange(&song, vrc6);
            let mut p = file::Project::new_empty();
            p.arrangement = imp.arrangement;
            p.bpm = imp.bpm.unwrap_or(p.bpm).clamp(40.0, 300.0);
            p.loop_end_ticks = imp.loop_end;
            let _ = p.arrangement.song_snapshot(1);
            let _ = export::render_limited(&p, 0.05);
            // What the import writes must load back.
            file::from_str(&file::to_string(&p)).expect("an imported song saves and loads");
        }
    }
}

#[test]
fn midi_parser_never_panics_or_hangs() {
    let seeds = midi_seeds();
    for s in &seeds {
        midi::parse(s).expect("every MIDI seed is valid");
    }
    campaign("midi", &seeds, b"MThd\0\0\0\x06\0\x01", 0x4D49_4449, check_midi);
}

// ───────────────────────────── WAV ─────────────────────────────



fn wav_seeds() -> Vec<Vec<u8>> {
    let mut seeds = Vec::new();
    for (format, bits) in [(1, 8), (1, 16), (1, 24), (1, 32), (3, 32), (0xFFFE, 16)] {
        for (ch, rate) in [(1, 44_100), (2, 48_000), (2, 96_000)] {
            seeds.push(wav(format, ch, rate, bits, &tone(0.05, rate)));
        }
    }
    seeds.push(wav(1, 1, 44_100, 16, &[0.0; 2000]));
    seeds.push(wav(1, 1, 44_100, 16, &(0..2000).map(|i| if i % 40 < 20 { 1.0 } else { -1.0 }).collect::<Vec<_>>()));
    seeds.push(wav(3, 1, 48_000, 32, &[f32::NAN, f32::INFINITY, -f32::INFINITY, 1e30, 0.5, 0.0]));
    seeds
}

fn check_wav(b: Vec<u8>) {
    if let Ok((pcm, sr)) = dpcm::read_wav(&b) {
        for free in [chip::samples::USER_ROM, 100, 17, 0] {
            if let Ok((s, _)) = dpcm::convert("fuzz", &pcm, sr, free) {
                assert!(s.bytes.len() <= free.min(chip::samples::MAX_SAMPLE_BYTES));
                assert_eq!((s.bytes.len() - 1) % 16, 0, "playable length");
                let _ = dpcm::bank(&[Some(s.clone()), Some(s.clone()), Some(s.clone()), Some(s)]);
            }
        }
    }
}

#[test]
fn wav_reader_never_panics_or_hangs() {
    let seeds = wav_seeds();
    for s in &seeds {
        dpcm::read_wav(s).expect("every WAV seed is valid");
    }
    campaign("wav", &seeds, b"RIFF\0\0\0\0WAVE", 0x5741_5645, check_wav);
}

// ───────────────────────────── .chipdaw ─────────────────────────────

fn project_seeds() -> Vec<Vec<u8>> {
    let mut demo = file::Project::new_empty();
    demo.arrangement = mock::demo_arrangement();
    let mut heavy = file::Project::new_empty();
    heavy.arrangement = mock::heavy_arrangement();
    let (pcm, sr) = dpcm::read_wav(&wav(1, 1, 44_100, 16, &tone(0.2, 44_100))).unwrap();
    let (s, _) = dpcm::convert("hit", &pcm, sr, chip::samples::USER_ROM).unwrap();
    heavy.arrangement.samples = vec![Some(s.clone()), None, Some(s), None];
    let mut stress = file::Project::new_empty();
    stress.arrangement = mock::stress_arrangement();
    // Version 3 instrument table with clip overrides (one shared, one own).
    let mut overrides = file::Project::new_empty();
    overrides.arrangement = mock::demo_arrangement();
    let a = &mut overrides.arrangement;
    let cello = a.instruments.add(ui::instruments::LaneInstrument::named("Cello")).unwrap();
    let kit = a.instruments.add(ui::instruments::LaneInstrument::named("Tight Kit")).unwrap();
    let ids: Vec<(u32, usize)> = a.clips.iter().map(|c| (c.id, c.lane)).collect();
    for (id, lane) in ids {
        let inst = match lane {
            0 | 1 | 5 | 6 => Some(cello),
            3 => Some(kit),
            _ => None,
        };
        a.clip_mut(id).unwrap().instrument = inst;
    }
    [file::Project::new_empty(), demo, heavy, stress, overrides]
        .iter()
        .map(|p| file::to_string(p).into_bytes())
        .collect()
}

/// JSON-aware mutation: replace numbers with extremes, drop or repeat lines.
fn mutate_json(seed: &[u8], rng: &mut Rng) -> Vec<u8> {
    let text = String::from_utf8_lossy(seed);
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    for _ in 0..1 + rng.below(4) {
        if lines.is_empty() {
            break;
        }
        let i = rng.below(lines.len());
        match rng.below(5) {
            0 | 1 => {
                // A number in the line becomes an extreme value.
                let extremes = ["4294967295", "4294967296", "-1", "0", "255", "256", "1e40", "-1e40", "18446744073709551615", "65535", "127", "-128", "3.5"];
                let l = &lines[i];
                let starts: Vec<usize> = l
                    .char_indices()
                    .filter(|&(k, c)| c.is_ascii_digit() && (k == 0 || !l.as_bytes()[k - 1].is_ascii_digit()))
                    .map(|(k, _)| k)
                    .collect();
                if let Some(&s) = starts.get(rng.below(starts.len().max(1))) {
                    let e = l[s..].find(|c: char| !c.is_ascii_digit()).map_or(l.len(), |e| s + e);
                    let x = extremes[rng.below(extremes.len())];
                    lines[i] = format!("{}{}{}", &l[..s], x, &l[e..]);
                }
            }
            2 => {
                lines.remove(i);
            }
            3 => {
                let l = lines[i].clone();
                lines.insert(i, l);
            }
            _ => {
                // Strings: non-ASCII in names and sample data.
                lines[i] = lines[i].replacen("\"", "\"é", 1);
            }
        }
    }
    lines.join("\n").into_bytes()
}

fn check_project(b: Vec<u8>) {
    let Ok(text) = std::str::from_utf8(&b) else { return };
    if let Ok(p) = file::from_str(text) {
        let _ = p.arrangement.song_snapshot(1);
        let _ = dpcm::free_bytes(&p.arrangement.samples);
        let _ = export::render_limited(&p, 0.05);
        // Whatever loads must save and load again unchanged.
        let again = file::from_str(&file::to_string(&p)).expect("a loaded project saves and loads");
        assert_eq!(file::to_string(&again), file::to_string(&p));
    }
}

#[test]
fn project_loader_never_panics_or_hangs() {
    let seeds = project_seeds();
    for s in &seeds {
        file::from_str(std::str::from_utf8(s).unwrap()).expect("every project seed is valid");
    }
    // Half byte-level, half JSON-aware mutations.
    let mut rng = Rng(0xC41D_A111);
    let json: Vec<Vec<u8>> = (0..count() / 2).map(|_| mutate_json(&seeds[rng.below(seeds.len())], &mut rng)).collect();
    let mut failures = Vec::new();
    for (i, input) in json.into_iter().enumerate() {
        let copy = input.clone();
        if let Some(msg) = guarded(Duration::from_secs(20), move || check_project(copy)) {
            failures.push(format!("json-mutated #{i}: {msg}"));
            if std::env::var("CHIPDAW_FUZZ_DUMP").is_ok() {
                let _ = std::fs::write(format!("fuzz_json_{i}.chipdaw"), &input);
            }
            if failures.len() >= 20 {
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
    campaign("chipdaw", &seeds, b"{\"format\": \"chipdaw\", \"version\": 2", 0x4348_4950, check_project);
}

#[test]
fn midi_notes_past_the_song_limit_are_dropped_and_reported() {
    // 40 maximal deltas (0x0FFFFFFF each) at 96 PPQ: ~10^11 song ticks.
    let mut ev: Vec<(u32, Vec<u8>)> = vec![(0, vec![0x90, 60, 100]), (96, vec![0x80, 60, 0])];
    for _ in 0..40 {
        ev.push((0x0FFF_FFFF, vec![0xB0, 7, 100]));
    }
    ev.push((0, vec![0x90, 62, 100]));
    ev.push((96, vec![0x80, 62, 0]));
    let song = midi::parse(&smf(1, 96, &[trk(&ev)])).unwrap();
    let imp = midi::arrange(&song, true);
    assert!(imp.report.contains("dropped"), "{}", imp.report);
    let notes: usize = imp.arrangement.clips.iter().map(|c| c.notes.len()).sum();
    assert_eq!(notes, 1);
    assert!(imp.loop_end <= ui::state::MAX_SONG_TICKS);
}

#[test]
fn midi_unclosed_notes_end_with_their_track() {
    let song = midi::parse(&smf(1, 480, &[trk(&[(0, vec![0x90, 60, 100]), (960, vec![0xB0, 7, 1])])])).unwrap();
    let imp = midi::arrange(&song, true);
    let c = &imp.arrangement.clips[0];
    assert_eq!((c.notes[0].start, c.notes[0].len), (0, 1920));
}

/// One regression test per crash found by the campaign (or by reading the
/// parsers while writing it): each used to panic or misbehave, now errors.
#[test]
fn known_crashers_are_errors_now() {
    let base = String::from_utf8(project_seeds()[1].clone()).unwrap();
    let edit = |from: &str, to: &str| {
        assert!(base.contains(from), "{from}");
        file::from_str(&base.replacen(from, to, 1))
    };
    // A clip id at u32::MAX: `id + 1` overflowed (found: state.rs).
    assert!(edit("\"id\": 1,", "\"id\": 4294967295,").unwrap_err().contains("out of range"));
    // A note at the end of time: `start + len` overflowed (found: notes.rs).
    let note = base.find("{ \"id\": 1, \"start\": ").unwrap();
    let start = &base[note..note + 40];
    let start_field = &start[..start.find("\"len\"").unwrap()];
    assert!(edit(start_field, "{ \"id\": 1, \"start\": 4294967295, ").unwrap_err().contains("ends after bar"));
    // A clip past the song limit.
    assert!(edit("\"start\": 0,", "\"start\": 4294000000,").unwrap_err().contains("ends after bar"));
    // A huge loop end is clamped, so an export can't ask for terabytes.
    let p = edit("\"loop_end\": ", "\"loop_end\": 4294967295, \"x_\": ").map(|_| ()).unwrap_err();
    assert!(p.contains("unknown field"), "{p}");
    let p = file::from_str(&base.replacen(&format!("\"loop_end\": {}", 32 * ui::state::TICKS_PER_BAR), "\"loop_end\": 4294967295", 1)).unwrap();
    assert_eq!(p.loop_end_ticks, ui::state::MAX_SONG_TICKS);
    // Non-ASCII in sample data: slicing the hex string at byte offsets panicked.
    assert!(dpcm::from_hex("aéa").is_err());
    assert!(dpcm::from_hex("0é").is_err());
    assert_eq!(dpcm::from_hex("00ff7F").unwrap(), [0, 255, 127]);
    // Samples that overflow the sample ROM: `free_bytes` underflowed.
    let big = dpcm::UserSample { name: "x".into(), rate: 15, bytes: vec![0; chip::samples::MAX_SAMPLE_BYTES] };
    assert_eq!(dpcm::free_bytes(&[Some(big.clone()), Some(big.clone()), Some(big.clone()), Some(big)]), 0);
    let mut heavy = String::from_utf8(project_seeds()[2].clone()).unwrap();
    let data = format!("\"data\": \"{}\"", "00".repeat(chip::samples::MAX_SAMPLE_BYTES));
    let mut from = 0;
    while let Some(i) = heavy[from..].find("\"data\": \"") {
        let s = from + i;
        let e = s + 9 + heavy[s + 9..].find('"').unwrap() + 1;
        heavy.replace_range(s..e, &data);
        from = s + data.len();
    }
    let heavy = heavy.replacen("\"slot\": 2", "\"slot\": 1", 1).replacen("\"slot\": 0", "\"slot\": 3", 1);
    let r = file::from_str(&heavy);
    assert!(r.as_ref().is_err_and(|e| e.contains("don't fit")) || r.is_ok_and(|p| dpcm::used_bytes(&p.arrangement.samples) <= chip::samples::USER_ROM));
    // Float WAV with NaN / infinity: read as silence, never NaN.
    let (pcm, _) = dpcm::read_wav(&wav(3, 1, 48_000, 32, &[f32::NAN, f32::INFINITY, 0.5])).unwrap();
    assert!(pcm.iter().all(|x| x.is_finite()));
    // A very long WAV is cut at read time, and the conversion says so.
    let long = wav(1, 1, 8_000, 16, &tone(30.0, 8_000));
    let (pcm, sr) = dpcm::read_wav(&long).unwrap();
    assert_eq!(pcm.len(), (sr * dpcm::MAX_IMPORT_SECONDS) as usize);
    assert!(dpcm::convert("long", &pcm, sr, chip::samples::USER_ROM).unwrap().1, "reports the cut");
}
