//! Renders a `.mid` file through the app's own MIDI import (Batch 5 A4: the
//! private reference renders, by ear only; the tool itself names no song and
//! writes only where it is told).
//!
//! usage: cargo run --release -p ui --example midi_render -- <in.mid> <out.wav>
//!        --profile vrc6|2a03|none [--studio]
//!
//! - With a profile and no `--studio`: the parts are imported with **Fit to
//!   hardware** (locked tracks on the profile's free channels, notes folded
//!   into range, chords reduced), and each locked track gets an **authentic**
//!   preset of its channel's voice from the Hardware folder.
//! - With `--studio` (or `--profile none`): the parts arrive as **studio
//!   tracks** and each gets a Sounds / Drums preset that fits its voice
//!   (triangle and low parts: Basses; pulse: Leads; noise: Drums kits; DPCM:
//!   the sample map).
//!
//! The presets are picked in turn from each folder, so two tracks of one voice
//! sound different. It prints which preset went on which track.

use std::path::Path;

use chip::voice::VoiceKind;
use ui::bank;
use ui::file::Project;
use ui::midi_tracks::{self, Mode, Target};
use ui::state::{SoundChip, PPQ};
use ui::theme::tokens::Rgba;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: midi_render <in.mid> <out.wav> --profile vrc6|2a03|none [--studio]");
        std::process::exit(2);
    }
    let (input, output) = (Path::new(&args[1]), Path::new(&args[2]));
    let flag = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1)).map(|s| s.as_str());
    let profile = match flag("--profile").unwrap_or("none") {
        "vrc6" => SoundChip::WithVrc6,
        "2a03" => SoundChip::Base2A03,
        _ => SoundChip::StudioOnly,
    };
    let studio = args.iter().any(|a| a == "--studio") || profile == SoundChip::StudioOnly;

    let bytes = std::fs::read(input).expect("read the .mid");
    let song = ui::midi::parse(&bytes).expect("parse the .mid");
    let parts = midi_tracks::plan(&song);
    let bpm = song.bpm.unwrap_or(120.0).clamp(20.0, 999.0);

    let mut pr = Project::new_empty();
    pr.arrangement.lanes.clear();
    pr.arrangement.instruments.lanes.clear();
    pr.chip = profile;
    pr.filter = ui::tracks::default_filter(profile);
    pr.bpm = bpm;
    let mode = if studio { Mode::Studio } else { Mode::Fit { arp: false } };
    let mut colour = |_k: VoiceKind| Rgba::hex(0x4FA8E0);
    let (report, _lanes) = midi_tracks::import(&mut pr.arrangement, &parts, 0, Target::End, mode, profile, bpm, &mut colour);
    println!("import ({}, {} parts, {bpm:.1} BPM): {report}", if studio { "studio" } else { "fit" }, parts.len());

    // Presets, in turn per folder.
    let all = bank::all();
    let mut used: std::collections::HashMap<String, usize> = Default::default();
    for lane in 0..pr.arrangement.lanes.len() {
        let t = &pr.arrangement.lanes[lane];
        let (voice, locked) = (t.voice, t.lock.is_some());
        let cands: Vec<&bank::Preset> = if locked {
            all.iter().filter(|p| p.profile.is_some_and(|pp| pp == profile || pp == SoundChip::Base2A03) && p.voice == voice).collect()
        } else {
            let role = match voice {
                VoiceKind::Triangle => "Basses",
                VoiceKind::Noise => "Kits",
                VoiceKind::Dpcm => "Hits",
                _ => "Leads",
            };
            all.iter().filter(|p| !p.authentic() && !p.studio_only() && p.path.iter().any(|s| s == role) && p.fits_voice(voice)).collect()
        };
        if cands.is_empty() {
            println!("track {lane} ({voice:?}{}): no preset fits, plain channel", if locked { ", locked" } else { "" });
            continue;
        }
        let key = format!("{voice:?}{locked}");
        let k = used.entry(key).or_insert(0);
        let p = cands[*k % cands.len()];
        *k += 1;
        bank::apply(&mut pr.arrangement, lane, p);
        println!("track {lane} \"{}\" ({voice:?}{}): {} [{}]", pr.arrangement.lanes[lane].name, if locked { ", locked" } else { "" }, p.name, p.folder());
    }
    // The whole song once.
    let end = pr.arrangement.clips.iter().map(|c| c.start_ticks + c.len_ticks).max().unwrap_or(PPQ * 4);
    pr.loop_end_ticks = end;
    let (x, ch) = ui::export::render_channels(&pr);
    ui::export::write_wav_ch(&x, ch, output).expect("write the WAV");
    let secs = x.len() as f64 / ch as f64 / 48_000.0;
    let lufs = ui::game_export::measure_ch(&x, 48_000, ch).map_or(f64::NEG_INFINITY, |m| m.0);
    println!("wrote {} ({secs:.1} s, {ch} ch, {lufs:.1} LUFS)", output.display());
}
