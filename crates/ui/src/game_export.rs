//! Game export (STABILIZE_PROMPT.md Part C): seamless loops, stems and an
//! optional loudness stage, rendered offline through the same emulation the
//! app plays (never on the audio thread).
//!
//! **Seamless loops.** An emulator doesn't repeat itself exactly: its 60.1 Hz
//! driver clock, the noise LFSR, oscillator phases and filter state run on
//! through a loop, so the third time through differs slightly from the
//! second. The export therefore renders the intro and the first pass
//! normally, takes a copy of the *entire* emulator state at the start of the
//! second pass (`Player: Clone`), and plays every later pass from that copy.
//! The loop is then exactly periodic, and its start carries the release
//! tails, envelopes, filter state and LFSR that ran past the loop end, as
//! continuous playback has them. The first pass can't carry those tails (it
//! follows the intro), so the "intro" part of an export runs up to the
//! second pass: `intro.wav` = intro + first pass, `loop.wav` = the second
//! pass. Then `intro + loop × n` is sample-identical to a continuous render
//! (`render`, tested in `tests/export_loop.rs`).
//!
//! Rates: the emulator renders natively at any rate (its band-limited step
//! synthesis and output filters are built for the rate), so 44.1 kHz needs
//! no resampling.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};


use crate::file::Project;
use crate::state::{PPQ, TICKS_PER_BAR};

/// Release tail after a song or selection export (not after a loop: its
/// tail is baked into the loop start).
pub const TAIL_SECONDS: f64 = 2.0;
const CHUNK: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Range {
    /// From the start to the song's loop end, plus the release tail.
    Song,
    /// The loop region as a seamless loop (intro + loop).
    Loop,
    /// A time range (ticks), plus the release tail.
    Selection { start: u32, end: u32 },
}

/// A user-defined stem: a name and the lanes it plays (bit per lane).
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub name: String,
    pub lanes: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stems {
    None,
    /// One stem per channel (8).
    Channels,
    Groups(Vec<Group>),
}

/// The default groups: melody (both pulses and the VRC6 pulses), bass
/// (triangle and saw), drums (noise and DPCM).
pub fn default_groups() -> Vec<Group> {
    vec![
        Group { name: "melody".into(), lanes: 0b0110_0011 },
        Group { name: "bass".into(), lanes: 0b1000_0100 },
        Group { name: "drums".into(), lanes: 0b0001_1000 },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Loudness {
    /// The chip's own level, untouched (the default).
    Authentic,
    /// One gain so the mix peaks at `dbfs` (sample peak).
    Peak { dbfs: f64 },
    /// One gain so the mix measures `lufs` (EBU R128 integrated), lowered if
    /// needed so the true peak stays under `ceiling_dbtp`.
    Lufs { lufs: f64, ceiling_dbtp: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub range: Range,
    pub rate: u32,
    /// 16 or 24.
    pub bits: u16,
    /// One WAV with the loop in a `smpl` chunk and cue markers.
    pub single: bool,
    /// `_intro.wav` + `_loop.wav` (loop range only).
    pub split: bool,
    /// A JSON sidecar with the loop points (loop range only).
    pub json: bool,
    pub stems: Stems,
    pub loudness: Loudness,
}

impl Default for Options {
    fn default() -> Self {
        Self { range: Range::Loop, rate: 48_000, bits: 16, single: true, split: true, json: true, stems: Stems::None, loudness: Loudness::Authentic }
    }
}

/// Where things are in a rendered track (samples).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub total: usize,
    /// The loop in the files: start (inclusive) and end (exclusive). It is
    /// the second pass; `start` is also the length of `_intro.wav`.
    pub loop_: Option<(usize, usize)>,
    /// Where the loop region musically starts (the end of the intro proper).
    pub musical_loop_start: usize,
    /// 1 (mono) or 2 (stereo, interleaved): `tracks::channels`. Every
    /// position above counts frames.
    pub channels: u16,
}

/// Progress (0..=1000) and cancel flag, shared with the UI thread.
#[derive(Default)]
pub struct Control {
    pub permille: AtomicU32,
    pub cancel: AtomicBool,
}

pub const CANCELLED: &str = "Export cancelled.";

/// Which tracks a render plays: `mask` (bit per console lane type: a
/// track is in when its lane's bit is set; 0 = the project's own mute and
/// solo).
pub fn heard_for_mask(p: &Project, mask: u8) -> Vec<bool> {
    if mask == 0 {
        crate::tracks::audible(&p.arrangement, &p.mute, &p.solo)
    } else {
        p.arrangement.lanes.iter().map(|l| mask & (1 << l.chan()) != 0).collect()
    }
}

fn ticks_to_samples(ticks: f64, bpm: f64, rate: u32) -> usize {
    (ticks / (bpm / 60.0 * PPQ as f64) * rate as f64).round() as usize
}

/// The loop region in ticks: the song loop (start..end).
pub fn loop_ticks(p: &Project) -> (u32, u32) {
    let end = p.loop_end_ticks.max(PPQ);
    (p.loop_start_ticks.min(end - PPQ), end)
}

/// Renders one track (the mix, or a stem with `solo`, a lane-type mask:
/// `heard_for_mask`) for `range`. Loops: intro, first pass, then
/// `passes - 1` passes replayed from the state at the start of the second
/// pass.
pub fn render(p: &Project, rate: u32, solo: u8, range: Range, passes: usize, ctl: Option<(&Control, f64, f64)>) -> Result<(Vec<f32>, Layout), String> {
    render_heard(p, rate, &heard_for_mask(p, solo), range, passes, ctl)
}

/// `render` with the tracks heard given one by one (a stem per track).
pub fn render_heard(p: &Project, rate: u32, heard: &[bool], range: Range, passes: usize, ctl: Option<(&Control, f64, f64)>) -> Result<(Vec<f32>, Layout), String> {
    let (mut m, snap) = crate::tracks::offline_mixdown(p, rate, heard, true);
    let bpm = p.bpm as f64;
    // Stereo once a studio track is panned (the mix and every stem alike, so
    // stems still line up with the mix); mono otherwise, exactly as before.
    let channels = crate::tracks::channels(&p.arrangement);
    // Progress within this track's share of the whole export.
    let report = |done: usize, total: usize| -> Result<(), String> {
        if let Some((c, from, span)) = ctl {
            if c.cancel.load(Ordering::Relaxed) {
                return Err(CANCELLED.into());
            }
            let f = from + span * done as f64 / total.max(1) as f64;
            c.permille.store((f * 1000.0).min(1000.0) as u32, Ordering::Relaxed);
        }
        Ok(())
    };
    let run = |pl: &mut engine::Mixdown, out: &mut Vec<f32>, n: usize, base: usize, total: usize| -> Result<(), String> {
        let mut done = 0;
        let (mut l, mut r) = (vec![0f32; CHUNK], vec![0f32; CHUNK]);
        while done < n {
            let k = CHUNK.min(n - done);
            if channels == 1 {
                let at = out.len();
                out.resize(at + k, 0.0);
                pl.render(&snap, &mut out[at..]);
            } else {
                pl.render_stereo(&snap, &mut l[..k], &mut r[..k]);
                out.extend(l[..k].iter().zip(&r[..k]).flat_map(|(a, b)| [*a, *b]));
            }
            done += k;
            report(base + done, total)?;
        }
        Ok(())
    };
    match range {
        Range::Loop => {
            let (ls, le) = loop_ticks(p);
            let intro = ticks_to_samples(ls as f64, bpm, rate);
            let len = ticks_to_samples((le - ls) as f64, bpm, rate).max(1);
            let passes = passes.max(2);
            let total = intro + passes * len;
            m.player.set_loop_start(ls as f64);
            m.player.set_loop_end(le as f64);
            m.player.play();
            let mut out = Vec::with_capacity(total * channels as usize);
            run(&mut m, &mut out, intro + len, 0, total)?;
            // The latch: the whole emulator (console, studio voices, Main) at
            // the start of the second pass.
            let latch = m.clone();
            for k in 1..passes {
                let mut q = latch.clone();
                run(&mut q, &mut out, len, intro + k * len, total)?;
            }
            Ok((out, Layout { total, loop_: Some((intro + len, intro + 2 * len)), musical_loop_start: intro, channels }))
        }
        Range::Song | Range::Selection { .. } => {
            let (start, end) = match range {
                Range::Selection { start, end } => (start, end.max(start + 1)),
                _ => (0, p.loop_end_ticks.max(PPQ)),
            };
            let body = ticks_to_samples((end - start) as f64, bpm, rate);
            let tail = (TAIL_SECONDS * rate as f64) as usize;
            let total = body + tail;
            m.player.set_loop_end(1.0e12);
            m.player.seek(start as f64);
            m.player.play();
            let mut out = Vec::with_capacity(total * channels as usize);
            run(&mut m, &mut out, body, 0, total)?;
            // Stop so held notes release and their tails ring out.
            m.player.stop();
            run(&mut m, &mut out, tail, body, total)?;
            Ok((out, Layout { total, loop_: None, musical_loop_start: 0, channels }))
        }
    }
}

// ───────────────────────────── loudness ─────────────────────────────

/// (integrated LUFS, true peak dBTP, sample peak dBFS) of mono audio.
pub fn measure(x: &[f32], rate: u32) -> Result<(f64, f64, f64), String> {
    measure_ch(x, rate, 1)
}

/// `measure` for `channels` interleaved channels (the true peak is the
/// louder channel's).
pub fn measure_ch(x: &[f32], rate: u32, channels: u16) -> Result<(f64, f64, f64), String> {
    use ebur128::{EbuR128, Mode};
    let ch = channels.max(1) as u32;
    let mut m = EbuR128::new(ch, rate, Mode::I | Mode::TRUE_PEAK).map_err(|e| format!("{e:?}"))?;
    m.add_frames_f32(x).map_err(|e| format!("{e:?}"))?;
    let lufs = m.loudness_global().map_err(|e| format!("{e:?}"))?;
    let mut tp = 0f64;
    for c in 0..ch {
        tp = tp.max(m.true_peak(c).map_err(|e| format!("{e:?}"))?);
    }
    let peak = x.iter().fold(0f32, |a, v| a.max(v.abs())) as f64;
    Ok((lufs, 20.0 * tp.max(1e-12).log10(), 20.0 * peak.max(1e-12).log10()))
}

/// The gain the loudness option asks for, measured on the mix. Returns
/// (gain, note for the report).
pub fn loudness_gain(mix: &[f32], rate: u32, l: Loudness) -> Result<(f32, String), String> {
    loudness_gain_ch(mix, rate, 1, l)
}

/// `loudness_gain` for `channels` interleaved channels.
pub fn loudness_gain_ch(mix: &[f32], rate: u32, channels: u16, l: Loudness) -> Result<(f32, String), String> {
    match l {
        Loudness::Authentic => Ok((1.0, "authentic chip level (no gain)".into())),
        Loudness::Peak { dbfs } => {
            let peak = mix.iter().fold(0f32, |a, v| a.max(v.abs())).max(1e-9);
            let g = 10f64.powf(dbfs / 20.0) as f32 / peak;
            Ok((g, format!("peak normalised to {dbfs:.1} dBFS (gain {:+.2} dB)", 20.0 * (g as f64).log10())))
        }
        Loudness::Lufs { lufs, ceiling_dbtp } => {
            let (l0, tp0, _) = measure_ch(mix, rate, channels)?;
            if !l0.is_finite() {
                return Ok((1.0, "silent: no gain".into()));
            }
            let want = lufs - l0;
            let room = ceiling_dbtp - tp0;
            let db = want.min(room);
            let note = if db < want {
                format!(
                    "{:.1} LUFS (asked for {lufs:.1}): the true-peak ceiling of {ceiling_dbtp:.1} dBTP limits the gain to {db:+.2} dB",
                    l0 + db
                )
            } else {
                format!("{lufs:.1} LUFS integrated (gain {db:+.2} dB), true peak {:.1} dBTP", tp0 + db)
            };
            Ok((10f64.powf(db / 20.0) as f32, note))
        }
    }
}

// ───────────────────────────── WAV files ─────────────────────────────

/// Quantises to `bits` (16 or 24) signed integers, rounding, clamped.
pub fn quantize(x: f32, bits: u16) -> i32 {
    let full = if bits == 24 { 8_388_607.0 } else { 32_767.0 };
    let lo = if bits == 24 { -8_388_608 } else { -32_768 };
    ((x as f64 * full).round() as i64).clamp(lo, full as i64) as i32
}

fn chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

/// Mono PCM WAV. With a loop: a `smpl` chunk (one forward loop, end
/// inclusive as the format defines it, infinite) plus `cue ` points and
/// `LIST/adtl` labels "Loop Start" / "Loop End".
pub fn wav_bytes(x: &[f32], rate: u32, bits: u16, loop_: Option<(usize, usize)>) -> Vec<u8> {
    wav_bytes_ch(x, rate, bits, 1, loop_)
}

/// `wav_bytes` for `channels` interleaved channels; the loop is in frames.
pub fn wav_bytes_ch(x: &[f32], rate: u32, bits: u16, channels: u16, loop_: Option<(usize, usize)>) -> Vec<u8> {
    let width = (bits / 8) as usize;
    let ch = channels.max(1);
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&1u16.to_le_bytes());
    fmt.extend_from_slice(&ch.to_le_bytes());
    fmt.extend_from_slice(&rate.to_le_bytes());
    fmt.extend_from_slice(&(rate * width as u32 * ch as u32).to_le_bytes());
    fmt.extend_from_slice(&((width * ch as usize) as u16).to_le_bytes());
    fmt.extend_from_slice(&bits.to_le_bytes());
    let mut data = Vec::with_capacity(x.len() * width);
    for &s in x {
        data.extend_from_slice(&quantize(s, bits).to_le_bytes()[..width]);
    }
    let mut body = b"WAVE".to_vec();
    chunk(&mut body, b"fmt ", &fmt);
    if let Some((a, b)) = loop_ {
        let mut smpl = Vec::new();
        for v in [0u32, 0, 1_000_000_000 / rate, 60, 0, 0, 0, 1, 0] {
            smpl.extend_from_slice(&v.to_le_bytes());
        }
        for v in [1u32, 0, a as u32, (b - 1) as u32, 0, 0] {
            smpl.extend_from_slice(&v.to_le_bytes());
        }
        chunk(&mut body, b"smpl", &smpl);
        let mut cue = 2u32.to_le_bytes().to_vec();
        for (id, pos) in [(1u32, a as u32), (2, b as u32)] {
            cue.extend_from_slice(&id.to_le_bytes());
            cue.extend_from_slice(&pos.to_le_bytes());
            cue.extend_from_slice(b"data");
            cue.extend_from_slice(&0u32.to_le_bytes());
            cue.extend_from_slice(&0u32.to_le_bytes());
            cue.extend_from_slice(&pos.to_le_bytes());
        }
        chunk(&mut body, b"cue ", &cue);
        let mut adtl = b"adtl".to_vec();
        for (id, name) in [(1u32, "Loop Start"), (2, "Loop End")] {
            let mut l = id.to_le_bytes().to_vec();
            l.extend_from_slice(name.as_bytes());
            l.push(0);
            chunk(&mut adtl, b"labl", &l);
        }
        chunk(&mut body, b"LIST", &adtl);
    }
    chunk(&mut body, b"data", &data);
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// What a test (or a curious user) reads back from an exported WAV:
/// (samples as integers, rate, bits, smpl loop start..end exclusive).
pub type WavRead = (Vec<i32>, u32, u16, Option<(usize, usize)>);

pub fn read_wav(b: &[u8]) -> Result<WavRead, String> {
    if b.len() < 12 || &b[..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a WAV".into());
    }
    let (mut rate, mut bits, mut data, mut lp) = (0, 0, None, None);
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().unwrap()) as usize;
        let body = b.get(i + 8..i + 8 + len).ok_or("truncated chunk")?;
        match id {
            b"fmt " if len >= 16 => {
                rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
                bits = u16::from_le_bytes(body[14..16].try_into().unwrap());
            }
            b"smpl" if len >= 60 => {
                let at = |o: usize| u32::from_le_bytes(body[o..o + 4].try_into().unwrap()) as usize;
                lp = Some((at(44), at(48) + 1));
            }
            b"data" => data = Some(body),
            _ => {}
        }
        i += 8 + len + (len & 1);
    }
    let data = data.ok_or("no data")?;
    let w = (bits / 8) as usize;
    if w == 0 {
        return Err("no fmt".into());
    }
    let x = data
        .chunks_exact(w)
        .map(|c| if w == 3 { i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8 } else { i16::from_le_bytes([c[0], c[1]]) as i32 })
        .collect();
    Ok((x, rate, bits, lp))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("Can't write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Can't save {}: {e}", path.display()))
}

// ───────────────────────────── the whole export ─────────────────────────────

/// What an export did: files written and notes for the user.
#[derive(Debug, Default)]
pub struct Report {
    pub files: Vec<PathBuf>,
    pub notes: Vec<String>,
    pub layout: Option<Layout>,
}

/// The export note for studio-track automation (Batch 5): which tracks and
/// lanes are rendered into the audio but have no hardware equivalent.
/// `None` when no studio track is automated.
pub fn studio_automation_note(p: &Project) -> Option<String> {
    let a = &p.arrangement;
    let mut parts = Vec::new();
    for l in a.lanes.iter().filter(|l| l.lock.is_none()) {
        let owner = crate::automation::AutoOwner::Track(l.id);
        let names: Vec<String> = a
            .automation
            .automated(owner)
            .iter()
            .filter_map(|id| crate::automation::Automation::param_of(&a.lanes, owner, id).map(|p| p.name.to_string()))
            .collect();
        if !names.is_empty() {
            parts.push(format!("{} ({})", l.name, names.join(", ")));
        }
    }
    (!parts.is_empty()).then(|| {
        format!(
            "Studio-track automation is rendered into the audio but has no hardware equivalent (it can't play on a console): {}. Locked tracks' automation plays as the chip's per-frame register writes.",
            parts.join("; ")
        )
    })
}

/// File-name text: lower case, anything but letters and digits as "_".
fn file_word(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect()
}

/// Each rendered file and its name ("mix", a track or a group) with the
/// tracks it plays. Batch 4a: **stems are per track** (named after it; two
/// tracks with one name get "_2", "_3"…).
fn tracks(p: &Project, o: &Options) -> Vec<(String, Vec<bool>)> {
    let mut t = vec![("mix".to_string(), heard_for_mask(p, 0))];
    match &o.stems {
        Stems::None => {}
        Stems::Channels => {
            let n = p.arrangement.lanes.len();
            for (i, l) in p.arrangement.lanes.iter().enumerate() {
                let mut name = file_word(&l.name);
                let mut k = 2;
                while t.iter().any(|x| x.0 == name) {
                    name = format!("{}_{k}", file_word(&l.name));
                    k += 1;
                }
                t.push((name, (0..n).map(|j| j == i).collect()));
            }
        }
        Stems::Groups(g) => {
            for x in g.iter().filter(|x| x.lanes != 0) {
                t.push((file_word(&x.name), heard_for_mask(p, x.lanes)));
            }
        }
    }
    t
}

/// Renders and writes everything `o` asks for into `dir`, named after `base`.
/// Runs on a worker thread; `ctl` reports progress and can cancel.
pub fn export(p: &Project, o: &Options, dir: &Path, base: &str, ctl: &Control) -> Result<Report, String> {
    let tracks = tracks(p, o);
    let share = 1.0 / tracks.len() as f64;
    let mut rendered = Vec::with_capacity(tracks.len());
    for (i, (name, heard)) in tracks.iter().enumerate() {
        let (x, layout) = render_heard(p, o.rate, heard, o.range, 2, Some((ctl, i as f64 * share, share)))?;
        rendered.push((name.clone(), x, layout));
    }
    let layout = rendered[0].2;
    let ch = layout.channels.max(1);
    let (gain, note) = loudness_gain_ch(&rendered[0].1, o.rate, ch, o.loudness)?;
    let mut report = Report { layout: Some(layout), ..Default::default() };
    report.notes.push(note);
    // Batch 5: automation is in the audio. A locked track's plays as the
    // chip's per-frame register writes (as a game's sound driver would);
    // a studio track's has no hardware equivalent: said, never dropped.
    if let Some(n) = studio_automation_note(p) {
        report.notes.push(n);
    }
    if gain != 1.0 {
        for (_, x, _) in rendered.iter_mut() {
            for v in x.iter_mut() {
                *v *= gain;
            }
        }
    }
    let peak = rendered[0].1.iter().fold(0f32, |a, v| a.max(v.abs()));
    if peak > 1.0 {
        report.notes.push(format!("The mix peaks at {:+.1} dBFS and clips; pick a loudness option to fix it.", 20.0 * peak.log10()));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("Can't create {}: {e}", dir.display()))?;
    let stem = |name: &str| if name == "mix" { base.to_string() } else { format!("{base}_stem_{name}") };
    for (name, x, l) in &rendered {
        let n = stem(name);
        match l.loop_ {
            Some((a, b)) => {
                if o.single {
                    let f = dir.join(format!("{n}.wav"));
                    write(&f, &wav_bytes_ch(&x[..b * ch as usize], o.rate, o.bits, ch, Some((a, b))))?;
                    report.files.push(f);
                }
                if o.split {
                    let k = ch as usize;
                    for (suffix, part) in [("intro", &x[..a * k]), ("loop", &x[a * k..b * k])] {
                        let f = dir.join(format!("{n}_{suffix}.wav"));
                        write(&f, &wav_bytes_ch(part, o.rate, o.bits, ch, None))?;
                        report.files.push(f);
                    }
                }
            }
            None => {
                let f = dir.join(format!("{n}.wav"));
                write(&f, &wav_bytes_ch(x, o.rate, o.bits, ch, None))?;
                report.files.push(f);
            }
        }
    }
    if let (true, Some((a, b))) = (o.json, layout.loop_) {
        let (ls, le) = loop_ticks(p);
        let names: Vec<String> = rendered.iter().map(|(n, _, _)| stem(n)).collect();
        let j = serde_json::json!({
            "format": "cartwave-loop",
            "version": 1,
            "sample_rate": o.rate,
            "bits": o.bits,
            "channels": ch,
            "loop_start_sample": a,
            "loop_end_sample": b,
            "loop_length_samples": b - a,
            "intro_samples": a,
            "musical_loop_start_sample": layout.musical_loop_start,
            "bpm": p.bpm,
            "loop_start_bar": ls as f64 / TICKS_PER_BAR as f64 + 1.0,
            "loop_end_bar": le as f64 / TICKS_PER_BAR as f64 + 1.0,
            "bars_in_loop": (le - ls) as f64 / TICKS_PER_BAR as f64,
            "gain_db": 20.0 * (gain as f64).log10(),
            "master_gain_db": if p.master_db.is_finite() { serde_json::json!(p.master_db) } else { serde_json::json!("-inf") },
            "tracks": names,
            "files": {
                "single": o.single.then(|| format!("{base}.wav")),
                "intro": o.split.then(|| format!("{base}_intro.wav")),
                "loop": o.split.then(|| format!("{base}_loop.wav")),
            },
            "note": "Play from sample 0; when playback reaches loop_end_sample, continue at loop_start_sample. With the split files: play the intro once, then repeat the loop file.",
        });
        let f = dir.join(format!("{base}.json"));
        write(&f, serde_json::to_string_pretty(&j).map_err(|e| e.to_string())?.as_bytes())?;
        report.files.push(f);
    }
    ctl.permille.store(1000, Ordering::Relaxed);
    Ok(report)
}

/// The sum of the stems against the mix: (RMS error relative to the mix's
/// RMS, peak error in dBFS), both in dB. The 2A03's mixer is non-linear, so
/// solo renders don't add up to the mix.
pub fn sum_error(mix: &[f32], stems: &[&[f32]]) -> (f64, f64) {
    let (mut e2, mut m2, mut peak) = (0f64, 0f64, 0f64);
    for (i, &m) in mix.iter().enumerate() {
        let s: f64 = stems.iter().map(|x| x[i] as f64).sum();
        let d = s - m as f64;
        e2 += d * d;
        m2 += (m as f64) * (m as f64);
        peak = peak.max(d.abs());
    }
    (10.0 * (e2 / m2.max(1e-30)).log10(), 20.0 * peak.max(1e-12).log10())
}
