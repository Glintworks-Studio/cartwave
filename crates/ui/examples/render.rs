//! Command-line render: a project or MIDI file (plus optional WAV files to
//! import as DPCM samples) to a WAV file, through the same emulation the app
//! plays and exports with.
//!
//! cargo run --release -p ui --example render -- song.chipdaw out.wav
//! cargo run --release -p ui --example render -- tune.mid hey.wav out.wav

use std::path::Path;

use ui::{dpcm, export, file, midi};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (out, inputs) = args.split_last().ok_or("usage: render <song.chipdaw | tune.mid> [sample.wav ...] <out.wav>")?;
    let first = inputs.first().ok_or("no input file")?;
    let mut p = if first.ends_with(".mid") || first.ends_with(".midi") {
        let song = midi::parse(&std::fs::read(first).map_err(|e| e.to_string())?)?;
        let imp = midi::arrange(&song, true);
        println!("{}", imp.report);
        let mut p = file::Project::new_empty();
        p.arrangement = imp.arrangement;
        p.bpm = imp.bpm.unwrap_or(p.bpm);
        p.loop_end_ticks = imp.loop_end;
        p
    } else {
        file::load(Path::new(first))?
    };
    for w in &inputs[1..] {
        let (pcm, sr) = dpcm::read_wav(&std::fs::read(w).map_err(|e| e.to_string())?)?;
        let slot = dpcm::free_slot(&p.arrangement.samples).ok_or("no free sample slot")?;
        let name = Path::new(w).file_stem().map_or("sample".into(), |s| s.to_string_lossy().to_string());
        let (s, _) = dpcm::convert(&name, &pcm, sr, dpcm::free_bytes(&p.arrangement.samples))?;
        p.arrangement.samples.resize(chip::samples::USER_SLOTS, None);
        p.arrangement.samples[slot] = Some(s);
    }
    let t0 = std::time::Instant::now();
    let audio = export::render(&p);
    export::write_wav(&audio, Path::new(out))?;
    println!(
        "{:.1} s of audio rendered in {} ms -> {out}",
        audio.len() as f32 / export::SAMPLE_RATE as f32,
        t0.elapsed().as_millis()
    );
    Ok(())
}
