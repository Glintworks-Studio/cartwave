//! Standalone host for the ChipDAW GUI prototype.
//!
//! Optional: `--size WxH` sets the initial logical window size (useful for
//! capturing at 150%/200% display scaling on a 1440p screen).
//!
//! `--render-demo DIR [SECONDS]` renders the demo arrangement offline through
//! the chip engine (no audio device, no window) into `DIR/demo_excerpt_*.wav`
//! (NES filter, Famicom filter and raw), for the listening pack.

use vizia::prelude::*;

/// Every allocation is checked against the audio thread's no-allocation
/// guard: debug builds abort on a violation, release builds count it (shown in
/// the stress-test logs).
#[global_allocator]
static ALLOC: engine::alloc_check::CheckingAllocator = engine::alloc_check::CheckingAllocator;

const DEFAULT_SIZE: (u32, u32) = (1440, 900);

fn arg_after(flag: &str) -> Option<Vec<String>> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == flag)
        .map(|i| args[i + 1..].to_vec())
}

fn parse_size() -> (u32, u32) {
    arg_after("--size")
        .and_then(|a| a.first().cloned())
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or(DEFAULT_SIZE)
}

fn write_wav(path: &std::path::Path, x: &[f32], sr: u32) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let data_len = (x.len() * 2) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&sr.to_le_bytes())?;
    f.write_all(&(sr * 2).to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for &v in x {
        f.write_all(&((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes())?;
    }
    Ok(())
}

/// Offline render of the demo set through the same Player the engine uses.
fn render_demo(dir: &str, secs: f64) {
    use engine::chip::player::Player;
    let song = ui::mock::demo_arrangement().song_snapshot(1);
    let dir = std::path::Path::new(dir);
    std::fs::create_dir_all(dir).expect("output dir");
    for (suffix, mode) in [
        ("nes", engine::FilterMode::Nes),
        ("famicom", engine::FilterMode::Famicom),
        ("raw", engine::FilterMode::Off),
    ] {
        let mut p = Box::new(Player::new(48_000.0));
        p.set_bpm(150.0);
        p.synth.set_filter(mode);
        p.play();
        let n = (secs * 48_000.0) as usize;
        let mut out = vec![0f32; n];
        for chunk in out.chunks_mut(512) {
            p.render(&song.view(), chunk);
        }
        let peak = out.iter().fold(0f32, |m, v| m.max(v.abs()));
        let path = dir.join(format!("demo_excerpt_{suffix}.wav"));
        write_wav(&path, &out, 48_000).expect("write wav");
        println!(
            "{} ({secs} s, peak {peak:.3}, note-ons {:?})",
            path.display(),
            p.driver.note_ons
        );
    }
}

fn main() -> Result<(), ApplicationError> {
    if let Some(a) = arg_after("--render-demo") {
        let dir = a.first().map(String::as_str).unwrap_or("docs/audio");
        let secs = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(25.6);
        render_demo(dir, secs);
        return Ok(());
    }
    // winit only applies the minimum to user resizes, so clamp the start size too.
    let (w, h) = parse_size();
    let size = (w.max(ui::MIN_WINDOW_SIZE.0), h.max(ui::MIN_WINDOW_SIZE.1));
    Application::new(|cx| {
        ui::build_ui(cx);
    })
    .title("ChipDAW — GUI prototype")
    .inner_size(size)
    // Below this the top bar's sides and the lanes would be cut off.
    .min_inner_size(Some(ui::MIN_WINDOW_SIZE))
    // Present on vblank; the UI advances its model once per presented frame.
    // `CHIPDAW_VSYNC=0` turns it off (an experiment knob for PERF measurements).
    .vsync(std::env::var("CHIPDAW_VSYNC").map_or(true, |v| v != "0"))
    .run()
}
