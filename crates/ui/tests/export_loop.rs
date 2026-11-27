//! The game-export gates (STABILIZE_PROMPT.md §C1, §C2).
//!
//! C1: `intro + loop × 3` from the exported files is sample-identical to one
//! continuous render of the same length, at 44.1 and 48 kHz, 16 and 24 bit,
//! for the showcase and the test song. Native 44.1 kHz rendering is checked
//! by pitch. C2: stems have the loop's length and points; the sum of solo
//! stems against the mix is measured (it can't be zero: the mixer is
//! non-linear).

use ui::game_export::{self as gx, Options, Range, Stems};
use ui::state::TICKS_PER_BAR;
use ui::{dpcm, file, midi};

fn showcase() -> file::Project {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs");
    let song = midi::parse(&std::fs::read(root.join("showcase.mid")).unwrap()).unwrap();
    let imp = midi::arrange(&song, true);
    let mut p = file::Project::new_legacy();
    p.arrangement = imp.arrangement;
    p.bpm = imp.bpm.unwrap();
    p.loop_end_ticks = imp.loop_end;
    let (pcm, sr) = dpcm::read_wav(&std::fs::read(root.join("hey.wav")).unwrap()).unwrap();
    let (hey, _) = dpcm::convert("hey", &pcm, sr, chip::samples::USER_ROM).unwrap();
    p.arrangement.samples = vec![Some(hey), None, None, None];
    // Loop the last 8 bars: an 8-bar intro, then the loop.
    p.loop_start_ticks = p.loop_end_ticks - 8 * TICKS_PER_BAR;
    p
}

fn test_song() -> file::Project {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/scale_test.chipdaw");
    let mut p = file::load(&path).unwrap();
    // The whole song loops (no intro): the first pass is the "intro" file.
    p.loop_start_ticks = 0;
    p
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("cartwave-export-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn intro_plus_loop_three_times_is_sample_identical_to_a_continuous_render() {
    for (song, p) in [("showcase", showcase()), ("test_song", test_song())] {
        for rate in [44_100u32, 48_000] {
            for bits in [16u16, 24] {
                let o = Options { range: Range::Loop, rate, bits, ..Options::default() };
                let dir = tmp(&format!("{song}-{rate}-{bits}"));
                let ctl = gx::Control::default();
                let rep = gx::export(&p, &o, &dir, song, &ctl).unwrap();
                let (a, b) = rep.layout.unwrap().loop_.unwrap();
                let read = |f: &str| gx::read_wav(&std::fs::read(dir.join(f)).unwrap()).unwrap();
                let (intro, r1, b1, _) = read(&format!("{song}_intro.wav"));
                let (lp, _, _, _) = read(&format!("{song}_loop.wav"));
                assert_eq!((r1, b1), (rate, bits));
                assert_eq!((intro.len(), lp.len()), (a, b - a));
                // One continuous render of the same length: intro + 4 passes.
                let (cont, _) = gx::render(&p, rate, 0, Range::Loop, 4, None).unwrap();
                let cont: Vec<i32> = cont.iter().map(|&x| gx::quantize(x, bits)).collect();
                let mut joined = intro.clone();
                for _ in 0..3 {
                    joined.extend_from_slice(&lp);
                }
                assert_eq!(joined.len(), cont.len(), "{song} {rate} {bits}");
                let first_diff = joined.iter().zip(&cont).position(|(x, y)| x != y);
                assert_eq!(first_diff, None, "{song} {rate} Hz {bits}-bit: first difference at sample {first_diff:?}");
                // The single file: the same audio up to the loop end, with the
                // loop points in its smpl chunk.
                let (single, _, _, lpts) = read(&format!("{song}.wav"));
                assert_eq!(lpts, Some((a, b)));
                assert_eq!(single[..], joined[..b]);
                // The sidecar agrees.
                let j: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join(format!("{song}.json"))).unwrap()).unwrap();
                assert_eq!((j["loop_start_sample"].as_u64(), j["loop_end_sample"].as_u64()), (Some(a as u64), Some(b as u64)));
                assert_eq!(j["sample_rate"].as_u64(), Some(rate as u64));
                // The loop isn't silent and doesn't clip.
                let peak = lp.iter().map(|v| v.abs()).max().unwrap();
                let full = if bits == 24 { 8_388_607 } else { 32_767 };
                assert!(peak > full / 50 && peak < full, "{song} peak {peak}");
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }
}

/// Counts rising zero crossings of a 2A03 pulse A4 over 1 s at `rate`.
fn a4_frequency(rate: u32) -> f64 {
    use ui::notes::Note;
    let mut p = file::Project::new_legacy();
    p.loop_end_ticks = 16 * TICKS_PER_BAR;
    p.arrangement.instruments = ui::instruments::InstrumentTable::from_lanes(vec![ui::instruments::LaneInstrument::plain(); 8]);
    p.filter = engine::FilterMode::Off;
    let n = Note::new(1, 0, 8 * TICKS_PER_BAR, 69);
    p.arrangement.clips.push(std::sync::Arc::new(ui::state::Clip::new(1, 0, 0, 16 * TICKS_PER_BAR, "a".into(), vec![n])));
    let (x, _) = gx::render(&p, rate, 0, Range::Selection { start: 0, end: 4 * TICKS_PER_BAR }, 1, None).unwrap();
    // Skip the first 0.5 s, count over the next 1 s.
    let s = &x[rate as usize / 2..rate as usize * 3 / 2];
    let mean = s.iter().sum::<f32>() / s.len() as f32;
    let ups = s.windows(2).filter(|w| w[0] - mean <= 0.0 && w[1] - mean > 0.0).count();
    ups as f64
}

#[test]
fn the_engine_renders_natively_at_44_1_khz() {
    // The pulse period the driver picks for A4: f = CPU / (16 (t + 1)).
    let t = chip::pitch::period_for(chip::pitch::Osc::Pulse, 69).unwrap() as f64;
    let want = chip::tables::CPU_HZ_NTSC / (16.0 * (t + 1.0));
    for rate in [44_100, 48_000] {
        let f = a4_frequency(rate);
        assert!((f - want).abs() <= 1.0, "{rate} Hz: {f} crossings/s, want {want:.2}");
    }
}

#[test]
fn stems_share_the_loop_and_their_sum_error_is_measured() {
    let p = showcase();
    let o = Options { range: Range::Loop, rate: 48_000, bits: 16, stems: Stems::Channels, json: false, ..Options::default() };
    let dir = tmp("stems");
    let rep = gx::export(&p, &o, &dir, "show", &gx::Control::default()).unwrap();
    let (a, b) = rep.layout.unwrap().loop_.unwrap();
    let wavs: Vec<_> = rep.files.iter().filter(|f| f.to_string_lossy().ends_with("_stem_pulse_1.wav") || f.to_string_lossy().contains("_stem_")).collect();
    assert_eq!(wavs.len(), 8 * 3, "8 stems x (single + intro + loop)");
    for f in wavs.iter().filter(|f| !f.to_string_lossy().ends_with("_intro.wav") && !f.to_string_lossy().ends_with("_loop.wav")) {
        let (x, _, _, l) = gx::read_wav(&std::fs::read(f).unwrap()).unwrap();
        assert_eq!((x.len(), l), (b, Some((a, b))), "{}", f.display());
    }
    // Sum of the 8 solo stems vs the mix, over the whole render.
    let (mix, _) = gx::render(&p, 48_000, 0, Range::Loop, 2, None).unwrap();
    let stems: Vec<Vec<f32>> = (0..8).map(|l| gx::render(&p, 48_000, 1 << l, Range::Loop, 2, None).unwrap().0).collect();
    let refs: Vec<&[f32]> = stems.iter().map(|s| s.as_slice()).collect();
    let (rms_db, peak_db) = gx::sum_error(&mix, &refs);
    println!("channel stems: sum - mix = {rms_db:.1} dB RMS (relative to the mix), peak {peak_db:.1} dBFS");
    assert!(rms_db < -3.0 && rms_db > -80.0, "measurable but small: {rms_db}");
    let groups = gx::default_groups();
    let g: Vec<Vec<f32>> = groups.iter().map(|g| gx::render(&p, 48_000, g.lanes, Range::Loop, 2, None).unwrap().0).collect();
    let refs: Vec<&[f32]> = g.iter().map(|s| s.as_slice()).collect();
    let (grms, gpeak) = gx::sum_error(&mix, &refs);
    println!("group stems (melody/bass/drums): sum - mix = {grms:.1} dB RMS, peak {gpeak:.1} dBFS");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_latch_only_matters_after_the_second_pass() {
    // How far plain free-running playback drifts from the exported loop:
    // passes 3 and 4 rendered without the latch vs with it.
    let p = showcase();
    let rate = 48_000;
    let (latched, l) = gx::render(&p, rate, 0, Range::Loop, 4, None).unwrap();
    let (a, b) = l.loop_.unwrap();
    // Free-running: the same player, never restored.
    let snap = p.arrangement.song_snapshot(0);
    let mut pl = chip::player::Player::new(rate as f64);
    pl.set_bpm(p.bpm as f64);
    pl.set_vrc6(true);
    pl.synth.set_filter(p.filter);
    pl.driver.params = p.arrangement.console_params();
    pl.synth.rom[chip::samples::USER_BASE..].copy_from_slice(&snap.dpcm.data);
    pl.driver.set_user_samples(snap.dpcm.refs);
    let (ls, le) = gx::loop_ticks(&p);
    pl.set_loop_start(ls as f64);
    pl.set_loop_end(le as f64);
    pl.play();
    let mut free = vec![0f32; latched.len()];
    for c in free.chunks_mut(4096) {
        pl.render(&snap.view(), c);
    }
    assert_eq!(free[..b], latched[..b], "identical up to the end of the second pass");
    let tail = &latched[b..];
    let (rms, peak) = gx::sum_error(tail, &[&free[b..]]);
    println!("free-running passes 3-4 vs the exported loop: {rms:.1} dB RMS, peak {peak:.1} dBFS (loop {} samples)", b - a);
}
