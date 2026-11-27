//! Command-line game export: the same code as the app's export dialog.
//!
//! cargo run --release -p ui --example game_export -- [options] <song.cartwave | song.chipdaw | tune.mid [sample.wav ...]> <out dir>
//!
//! Options: --rate 44100|48000  --bits 16|24  --stems none|channels|groups
//!          --range loop|song  --loop-bars N (loop the last N bars)  --name base
//!          --loudness authentic|peak|lufs
//!
//! `docs/export-examples` was made with:
//!   ... -- --rate 44100 --loop-bars 8 --stems channels --name showcase docs/songs/showcase.mid docs/songs/hey.wav docs/export-examples

use std::path::Path;

use ui::game_export::{self as gx, Loudness, Options, Range, Stems};
use ui::{dpcm, file, midi};

fn main() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut o = Options { range: Range::Loop, ..Options::default() };
    let (mut name, mut loop_bars) = (None::<String>, None::<u32>);
    let mut i = 0;
    while i < args.len() {
        if !args[i].starts_with("--") {
            i += 1;
            continue;
        }
        let v = args.get(i + 1).cloned().ok_or(format!("{} needs a value", args[i]))?;
        match args[i].as_str() {
            "--rate" => o.rate = v.parse().map_err(|_| "bad --rate")?,
            "--bits" => o.bits = v.parse().map_err(|_| "bad --bits")?,
            "--stems" => {
                o.stems = match v.as_str() {
                    "channels" => Stems::Channels,
                    "groups" => Stems::Groups(gx::default_groups()),
                    _ => Stems::None,
                }
            }
            "--range" => o.range = if v == "song" { Range::Song } else { Range::Loop },
            "--loop-bars" => loop_bars = Some(v.parse().map_err(|_| "bad --loop-bars")?),
            "--name" => name = Some(v),
            "--loudness" => {
                o.loudness = match v.as_str() {
                    "peak" => Loudness::Peak { dbfs: -1.0 },
                    "lufs" => Loudness::Lufs { lufs: -16.0, ceiling_dbtp: -1.0 },
                    _ => Loudness::Authentic,
                }
            }
            other => return Err(format!("unknown option {other}")),
        }
        args.drain(i..i + 2);
    }
    let (out, inputs) = args.split_last().ok_or("usage: game_export [options] <song> [samples] <out dir>")?;
    let first = inputs.first().ok_or("no input")?;
    let mut p = if first.ends_with(".mid") || first.ends_with(".midi") {
        let imp = midi::arrange(&midi::parse(&std::fs::read(first).map_err(|e| e.to_string())?)?, true);
        let mut p = file::Project::new_legacy();
        p.arrangement = imp.arrangement;
        p.bpm = imp.bpm.unwrap_or(p.bpm);
        p.loop_end_ticks = imp.loop_end;
        p.loop_start_ticks = 0;
        p
    } else {
        file::load(Path::new(first))?
    };
    for w in &inputs[1..] {
        let (pcm, sr) = dpcm::read_wav(&std::fs::read(w).map_err(|e| e.to_string())?)?;
        let slot = dpcm::free_slot(&p.arrangement.samples).ok_or("no free sample slot")?;
        let n = Path::new(w).file_stem().map_or("sample".into(), |s| s.to_string_lossy().to_string());
        let (s, _) = dpcm::convert(&n, &pcm, sr, dpcm::free_bytes(&p.arrangement.samples))?;
        p.arrangement.samples.resize(chip::samples::USER_SLOTS, None);
        p.arrangement.samples[slot] = Some(s);
    }
    if let Some(b) = loop_bars {
        p.loop_start_ticks = p.loop_end_ticks.saturating_sub(b * ui::state::TICKS_PER_BAR);
    }
    let base = name.unwrap_or_else(|| Path::new(first).file_stem().map_or("song".into(), |s| s.to_string_lossy().to_string()));
    let t0 = std::time::Instant::now();
    let r = gx::export(&p, &o, Path::new(out), &base, &gx::Control::default())?;
    for f in &r.files {
        println!("{}", f.display());
    }
    if let Some(l) = r.layout {
        println!("layout: {l:?}");
    }
    println!("{} ({:.1} s)", r.notes.join("; "), t0.elapsed().as_secs_f32());
    Ok(())
}
