//! Behaviour of the importers, the exporter and the project files
//! (STABILIZE_PROMPT.md §A3), with generated test files.

mod common;
use common::*;
use ui::state::{PPQ, TICKS_PER_BAR};
use ui::{dpcm, export, file, midi, mock};

fn lane_notes(imp: &midi::Imported, lane: usize) -> Vec<(u32, u32, u8)> {
    let mut v: Vec<_> = imp
        .arrangement
        .clips
        .iter()
        .filter(|c| c.lane == lane)
        .flat_map(|c| c.notes.iter().map(move |n| (c.start_ticks + n.start, n.len, n.pitch)))
        .collect();
    v.sort();
    v
}

// ───────────────────────────── MIDI ─────────────────────────────

#[test]
fn midi_format_0_splits_channels_into_parts() {
    let ev = [vec![(0, tempo(120.0))], melody(0, &[(0, 480, 72), (0, 480, 74)]), melody(1, &[(0, 960, 48)])].concat();
    let imp = midi::arrange(&midi::parse(&smf(0, 480, &[trk(&ev)])).unwrap(), true);
    // Two parts: the lower one is the triangle bass, the other Pulse 1.
    // One track: the deltas run on, so channel 2 starts after channel 1's notes.
    assert_eq!(lane_notes(&imp, 2), [(1920, 1920, 48)]);
    assert_eq!(lane_notes(&imp, 0), [(0, 960, 72), (960, 960, 74)]);
    assert_eq!(imp.bpm.map(f32::round), Some(120.0));
}

#[test]
fn midi_format_1_tempo_changes_keep_the_first_and_say_so() {
    let t0 = trk(&[(0, tempo(100.0)), (480, tempo(160.0)), (480, tempo(80.0))]);
    let imp = midi::arrange(&midi::parse(&smf(1, 480, &[t0, trk(&melody(0, &[(0, 480, 60)]))])).unwrap(), true);
    assert_eq!(imp.bpm.map(f32::round), Some(100.0));
    assert!(imp.report.contains("2 tempo changes ignored"), "{}", imp.report);
}

#[test]
fn midi_running_status_and_velocity_zero_note_offs() {
    let t = vec![0, 0x90, 60, 100, 0x60, 60, 0, 0, 62, 100, 0x60, 62, 0, 0, 0xFF, 0x2F, 0];
    let imp = midi::arrange(&midi::parse(&smf(1, 96, &[t])).unwrap(), true);
    assert_eq!(lane_notes(&imp, 0), [(0, 960, 60), (960, 960, 62)]);
}

#[test]
fn midi_many_tracks_fill_the_channels_and_report_the_rest() {
    let tracks: Vec<_> = (0..12u8)
        .map(|t| trk(&[vec![(0, name(&format!("Part {t}")))], melody(t % 9, &[(0, 480, 60 + t)])].concat()))
        .collect();
    let imp = midi::arrange(&midi::parse(&smf(1, 480, &tracks)).unwrap(), true);
    let used: Vec<usize> = (0..8).filter(|&l| !lane_notes(&imp, l).is_empty()).collect();
    assert_eq!(used, [0, 1, 2, 5, 6, 7], "6 melodic channels with VRC6");
    assert_eq!(imp.report.matches("no free channel").count(), 6, "{}", imp.report);
    let base = midi::arrange(&midi::parse(&smf(1, 480, &tracks)).unwrap(), false);
    assert_eq!(base.report.matches("no free channel").count(), 9, "2A03 only: 3 channels");
}

#[test]
fn midi_chords_and_overlaps_become_one_note_at_a_time() {
    let ev = [
        (0, vec![0x90, 60, 100]),
        (0, vec![0x90, 64, 100]),
        (0, vec![0x90, 67, 100]),
        (240, vec![0x90, 72, 100]),
        (240, vec![0x80, 60, 0]),
        (0, vec![0x80, 64, 0]),
        (0, vec![0x80, 67, 0]),
        (240, vec![0x80, 72, 0]),
    ];
    let imp = midi::arrange(&midi::parse(&smf(1, 480, &[trk(&ev)])).unwrap(), true);
    let n = lane_notes(&imp, 0);
    assert_eq!(n, [(0, 480, 67), (480, 960, 72)], "top note of the chord, then the higher overlap");
    for w in n.windows(2) {
        assert!(w[0].0 + w[0].1 <= w[1].0, "monophonic");
    }
}

#[test]
fn midi_out_of_range_notes_fold_into_the_channel() {
    // Pulse 1 can't go below A1 (33); 12 is C0, 127 is G9.
    let imp = midi::arrange(&midi::parse(&smf(1, 480, &[trk(&melody(0, &[(0, 240, 12), (0, 240, 127), (0, 240, 60)]))])).unwrap(), true);
    let ui::notes::RowKind::Piano { play_lo, play_hi, .. } = ui::notes::RowKind::for_lane(0) else { unreachable!() };
    let n = lane_notes(&imp, 0);
    assert_eq!(n.len(), 3);
    for (_, _, p) in &n {
        assert!((play_lo..=play_hi).contains(p), "{p} outside {play_lo}..{play_hi}");
    }
    assert_eq!(n[0].2 % 12, 0, "still a C");
    assert!(imp.report.contains("2 notes moved by octaves"), "{}", imp.report);
}

#[test]
fn midi_zero_length_notes_play_for_one_tick() {
    let imp = midi::arrange(&midi::parse(&smf(1, 480, &[trk(&melody(9, &[(0, 0, 36), (480, 0, 38)]))])).unwrap(), true);
    assert_eq!(lane_notes(&imp, 3).len(), 2, "drum triggers with no length survive");
}

#[test]
fn midi_drums_go_to_the_kit_and_a_dpcm_track_to_the_samples() {
    let drums = trk(&[vec![(0, name("Kit"))], melody(9, &[(0, 120, 36), (120, 120, 38), (120, 120, 42), (120, 120, 49)])].concat());
    let dpcm = trk(&[vec![(0, name("DPCM hits"))], melody(2, &[(0, 240, 36), (0, 240, 40)])].concat());
    let imp = midi::arrange(&midi::parse(&smf(1, 480, &[drums, dpcm])).unwrap(), true);
    let kit: Vec<u8> = lane_notes(&imp, 3).iter().map(|n| n.2).collect();
    assert_eq!(kit, [40, 56, 72, 92], "kick, snare, hat, cymbal rows");
    let s: Vec<u8> = lane_notes(&imp, 4).iter().map(|n| n.2).collect();
    assert_eq!(s, [36, 40], "Kick row and User 1 row");
    assert!(lane_notes(&imp, 0).is_empty(), "the DPCM track isn't a melody");
}

#[test]
fn midi_truncated_and_corrupt_files_are_clear_errors() {
    let good = smf(1, 480, &[trk(&melody(0, &[(0, 480, 60), (0, 480, 62)]))]);
    for cut in [3, 10, 14, 20, good.len() - 3] {
        let e = midi::parse(&good[..cut]).map(|_| ()).unwrap_err();
        assert!(e.contains("ends early") || e.contains("MThd"), "{cut}: {e}");
    }
    assert!(midi::parse(b"RIFF....").unwrap_err().contains("Not a MIDI file"));
    let mut fmt2 = good.clone();
    fmt2[9] = 2;
    assert!(midi::parse(&fmt2).unwrap_err().contains("format 2"));
    let mut smpte = good.clone();
    smpte[12] = 0xE7;
    assert!(midi::parse(&smpte).unwrap_err().contains("SMPTE"));
}

#[test]
fn the_showcase_tune_imports_the_same() {
    let imp = midi::arrange(&midi::parse(&std::fs::read("../../docs/songs/showcase.mid").unwrap()).unwrap(), true);
    assert!(imp.report.contains("Lead → Pulse 1 (45 notes)"), "{}", imp.report);
    assert!(!imp.report.contains("moved by octaves") && !imp.report.contains("ignored"), "{}", imp.report);
}

// ───────────────────────────── WAV → DPCM ─────────────────────────────

#[test]
fn wav_every_supported_format_rate_and_layout_imports() {
    for (format, bits) in [(1u16, 8u16), (1, 16), (1, 24), (1, 32), (3, 32), (0xFFFE, 24)] {
        for rate in [44_100u32, 48_000, 96_000] {
            for ch in [1u16, 2] {
                let src = tone(0.25, rate);
                let (pcm, sr) = dpcm::read_wav(&wav(format, ch, rate, bits, &src)).unwrap();
                assert_eq!(sr, rate);
                assert_eq!(pcm.len(), src.len());
                if ch == 1 {
                    // Within the format's resolution (8-bit: 1/128, plus the writer's rounding).
                    let tol = if bits == 8 { 0.02 } else { 1e-4 };
                    assert!(pcm.iter().zip(&src).all(|(a, b)| (a - b).abs() < tol), "{format} {bits} {rate}");
                    let (s, cut) = dpcm::convert("t", &pcm, sr, chip::samples::USER_ROM).unwrap();
                    assert!(!cut && s.rate == 15, "0.25 s fits at the top rate");
                } else {
                    // The test files' right channel is inverted: the mono mix is silent.
                    assert!(pcm.iter().all(|x| x.abs() < 0.01), "{format} {bits} stereo mix");
                }
            }
        }
    }
}

#[test]
fn wav_silence_clipping_and_long_files() {
    let (pcm, sr) = dpcm::read_wav(&wav(1, 1, 44_100, 16, &[0.0; 4410])).unwrap();
    assert!(dpcm::convert("s", &pcm, sr, chip::samples::USER_ROM).unwrap_err().contains("silent"));
    // Full-scale square, clipped: still encodes, still in range.
    let sq: Vec<f32> = (0..4410).map(|i| if i % 50 < 25 { 1.5 } else { -1.5 }).collect();
    let (pcm, sr) = dpcm::read_wav(&wav(3, 1, 44_100, 32, &sq)).unwrap();
    assert!(pcm.iter().all(|x| x.abs() <= 1.0));
    dpcm::convert("sq", &pcm, sr, chip::samples::USER_ROM).unwrap();
    // A minute of audio: reading is capped, the conversion truncates and says so.
    let (pcm, sr) = dpcm::read_wav(&wav(1, 1, 48_000, 16, &tone(60.0, 48_000))).unwrap();
    assert_eq!(pcm.len(), 48_000 * dpcm::MAX_IMPORT_SECONDS as usize);
    let (s, cut) = dpcm::convert("long", &pcm, sr, chip::samples::USER_ROM).unwrap();
    assert!(cut && s.rate == 0 && s.bytes.len() <= chip::samples::MAX_SAMPLE_BYTES, "rate {} len {}", s.rate, s.bytes.len());
}

#[test]
fn wav_corrupt_headers_are_errors() {
    let good = wav(1, 1, 44_100, 16, &tone(0.01, 44_100));
    assert!(dpcm::read_wav(b"RIFF").is_err());
    assert!(dpcm::read_wav(&good[..30]).is_err(), "cut inside fmt");
    let i = good.windows(4).position(|w| w == b"fmt ").unwrap();
    let mut no_fmt = good.clone();
    no_fmt[i..i + 4].copy_from_slice(b"fmx ");
    assert!(dpcm::read_wav(&no_fmt).is_err());
    let mut rate0 = good.clone();
    rate0[i + 12..i + 16].copy_from_slice(&0u32.to_le_bytes());
    assert!(dpcm::read_wav(&rate0).is_err());
    let mut adpcm = good.clone();
    adpcm[i + 8..i + 10].copy_from_slice(&2u16.to_le_bytes());
    assert!(dpcm::read_wav(&adpcm).unwrap_err().contains("isn't supported"));
}

#[test]
fn wav_rom_full_is_refused_clearly() {
    let (pcm, sr) = dpcm::read_wav(&wav(1, 1, 44_100, 16, &tone(5.0, 44_100))).unwrap();
    let mut samples = Vec::new();
    loop {
        let free = dpcm::free_bytes(&samples);
        match dpcm::convert("x", &pcm, sr, free) {
            Ok((s, _)) => samples.push(Some(s)),
            Err(e) => {
                assert!(e.contains("memory is full"), "{e}");
                break;
            }
        }
        assert!(samples.len() <= 8, "never loops");
    }
    assert!(dpcm::used_bytes(&samples) <= chip::samples::USER_ROM);
}

// ───────────────────────────── export ─────────────────────────────

fn project(bars: u32) -> file::Project {
    let mut p = file::Project::new_legacy();
    p.arrangement = mock::demo_arrangement();
    p.loop_end_ticks = bars * TICKS_PER_BAR;
    p
}

#[test]
fn export_length_is_the_song_plus_the_tail_and_tails_ring() {
    for bars in [1, 32] {
        let p = project(bars);
        let audio = export::render(&p);
        let song = (bars * TICKS_PER_BAR) as f64 / PPQ as f64 * 60.0 / p.bpm as f64;
        let want = ((song + 2.0) * export::SAMPLE_RATE as f64) as usize;
        assert_eq!(audio.len(), want, "{bars} bars");
        assert!(audio.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
    }
    // A release tail rings past the song end: the demo's instruments have
    // release points, so the first 100 ms after the end isn't silent.
    let p = project(8);
    let audio = export::render(&p);
    let end = (8.0 * 4.0 * 60.0 / p.bpm as f64 * export::SAMPLE_RATE as f64) as usize;
    let after: f32 = audio[end..end + 4800].iter().map(|x| x.abs()).fold(0.0, f32::max);
    assert!(after > 0.001, "tail after the end: {after}");
}

// ───────────────────────────── files ─────────────────────────────

#[test]
fn files_v1_loads_and_v2_round_trips() {
    let v1 = std::fs::read_to_string("../../docs/songs/scale_test.chipdaw").unwrap();
    assert!(v1.contains("\"version\": 1"));
    let p = file::from_str(&v1).unwrap();
    assert!(p.arrangement.instruments.lane_instruments().iter().all(|i| i.name == "Raw Channel"), "v1 = raw channels");
    let text = file::to_string(&p);
    assert_eq!(file::to_string(&file::from_str(&text).unwrap()), text);
    let mut h = project(4);
    let (pcm, sr) = dpcm::read_wav(&wav(1, 1, 44_100, 16, &tone(0.2, 44_100))).unwrap();
    h.arrangement.samples = vec![None, Some(dpcm::convert("hey", &pcm, sr, chip::samples::USER_ROM).unwrap().0), None, None];
    let again = file::from_str(&file::to_string(&h)).unwrap();
    assert_eq!(again.arrangement.samples, h.arrangement.samples);
    assert_eq!(again.arrangement.instruments, h.arrangement.instruments);
}

#[test]
fn files_atomic_save_keeps_the_old_file_when_writing_fails() {
    let dir = std::env::temp_dir().join(format!("cartwave-atomic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("song.chipdaw");
    file::save(&project(4), &path).unwrap();
    let before = std::fs::read_to_string(&path).unwrap();
    // Simulated failure: the temporary file can't be created (a directory is
    // in its way), as with a full disk or a permission error.
    std::fs::create_dir_all(dir.join("song.chipdaw.tmp")).unwrap();
    let mut p2 = project(8);
    p2.bpm = 99.0;
    assert!(file::save(&p2, &path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before, "the old file is intact");
    assert!(file::load(&path).is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn files_crash_recovery_round_trip() {
    let dir = std::env::temp_dir().join(format!("cartwave-recovery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::env::set_var("CARTWAVE_DATA_DIR", &dir);
    let r = file::Recovery::new().unwrap();
    assert!(r.start_session().is_none(), "first start: nothing to recover");
    let mut p = project(4);
    p.bpm = 123.0;
    r.autosave(&p).unwrap();
    // "Crash": no end_session. The next start finds the marker and the file.
    let r2 = file::Recovery::new().unwrap();
    assert_eq!(r2.start_session(), Some(file::LastSession::Crashed));
    let back = file::load(&r2.file()).unwrap();
    assert_eq!(file::to_string(&back), file::to_string(&p));
    r2.end_session(&back);
    assert!(file::Recovery::new().unwrap().start_session().is_none(), "clean quit: no prompt");
    std::env::remove_var("CARTWAVE_DATA_DIR");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dpcm_all_four_user_slots_full_play_with_the_built_ins() {
    use ui::notes::Note;
    // Four samples that together fill the user ROM.
    let mut samples = Vec::new();
    for k in 0..4 {
        let free = dpcm::free_bytes(&samples);
        let share = free / (4 - k);
        let t: Vec<f32> = (0..44_100).map(|i| ((i as f32) * (0.02 + 0.01 * k as f32)).sin()).collect();
        let (s, _) = dpcm::convert(&format!("s{k}"), &t, 44_100, share).unwrap();
        samples.push(Some(s));
    }
    assert!(dpcm::free_bytes(&samples) < 1024, "the ROM is (nearly) full: {} free", dpcm::free_bytes(&samples));
    let mut p = project(4);
    p.arrangement.clips.retain(|c| c.lane != 4);
    p.arrangement.samples = samples;
    // Each of the 8 DPCM rows (4 built-in, 4 user) on its own beat, twice.
    let notes: Vec<Note> = (0..16).map(|i| Note::new(i + 1, i * PPQ, PPQ, 36 + (i % 8) as u8)).collect();
    p.arrangement.clips.push(std::sync::Arc::new(ui::state::Clip::new(999, 4, 0, 16 * PPQ, "dpcm".into(), notes)));
    p.solo = ui::tracks::TrackSet(vec![p.arrangement.lanes[4].id]);
    let t0 = std::time::Instant::now();
    let audio = export::render(&p);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let beat = (60.0 / p.bpm as f64 * export::SAMPLE_RATE as f64) as usize;
    for i in 0..8 {
        let w = &audio[i * beat + beat / 8..(i + 1) * beat - beat / 8];
        let swing = w.iter().fold(f32::MIN, |a, x| a.max(*x)) - w.iter().fold(f32::MAX, |a, x| a.min(*x));
        assert!(swing > 0.01, "row {i} is silent (swing {swing})");
    }
    // The bank lays them out without overlap, inside the user area.
    let b = dpcm::bank(&p.arrangement.samples);
    let mut spans: Vec<(usize, usize)> = b.refs.iter().filter(|r| r.present).map(|r| (r.addr_reg as usize * 64, r.len_reg as usize * 16 + 1)).collect();
    assert_eq!(spans.len(), 4);
    spans.sort();
    for w in spans.windows(2) {
        assert!(w[0].0 + w[0].1 <= w[1].0, "overlap {w:?}");
    }
    println!("8 DPCM rows, 4 user slots full: {:.1} s rendered in {ms:.1} ms", audio.len() as f64 / export::SAMPLE_RATE as f64);
}
