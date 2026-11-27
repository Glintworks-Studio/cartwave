//! Version 1 and 2 files, migrated to version 3 on load, must render
//! bit-identically to how the build that saved them rendered them
//! (STABILIZE_PROMPT.md §B4, the golden-hash method).
//!
//! `docs/songs/v2/*.chipdaw` were written by the product/v1 build (281ee66)
//! with `docs/songs/v2/v2corpus.rs.txt`, which also printed the hashes below
//! from that build: `render::render` (the golden test's render: song + 1 s)
//! and `export::render` (song + release tail, with the DPCM samples). The
//! version 1 file is the golden test song (`golden.rs`, same hash as v1).

use ui::{export, file, render};

/// (file, render::render hash, export::render hash), from the old build.
const V2: &[(&str, u64, u64)] = &[
    ("demo_band", 0x0c984c632801ef74, 0x5080530f47f64c64),
    ("volumes_mixed", 0xe1840708a7c60272, 0xd8359071fd0824f9),
    ("raw_volumes", 0x9ce78c6f9d53c5d2, 0x2f45a89d297f00f8),
    ("layout_0", 0xf32404eae4b93fa2, 0xc56c98aeea4a3098),
    ("layout_1", 0x99fb04b277dac084, 0x57720d21669d5fd0),
    ("layout_2", 0x88f50e7e43f9f59e, 0x74293fd8cb504efd),
    ("layout_3", 0xbbfece4fdee90c11, 0x3b7264132f74366f),
    ("layout_4", 0x43621e26f83e83e3, 0x1297fa4fcc0f77b7),
    ("showcase", 0x7ec54ac5dd1d130d, 0x075f22c0555c07c8),
];

/// Saved on purpose with a saw Rate (50) no note volume can express next to
/// per-note volumes: the migration approximates it and must say so.
const INEXACT: (&str, u64, u64) = ("saw_rate_50_inexact", 0x9f52229416ccc523, 0xae933e3dcb12e1a1);

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v2")
}

fn secs(p: &file::Project) -> f64 {
    p.loop_end_ticks as f64 / ui::state::PPQ as f64 * 60.0 / p.bpm as f64 + 1.0
}

#[test]
fn version_2_files_render_bit_identically_after_migration() {
    let mut bad = Vec::new();
    for &(name, want_r, want_e) in V2 {
        let p = file::load(&dir().join(format!("{name}.chipdaw"))).unwrap();
        let r = render::hash(&render::render(&p, secs(&p)));
        let e = render::hash(&export::render(&p));
        if (r, e) != (want_r, want_e) {
            bad.push(format!("{name}: render {r:#018x} (want {want_r:#018x}), export {e:#018x} (want {want_e:#018x})"));
        }
        // Saved again in the current version and reloaded: still the same sound.
        let again = file::from_str(&file::to_string(&p)).unwrap();
        assert!(file::to_string(&p).contains(&format!("\"version\": {}", file::VERSION)));
        assert_eq!(render::hash(&export::render(&again)), e, "{name}: v3 round trip");
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn the_inexact_case_is_reported_not_hidden() {
    let (name, want_r, _) = INEXACT;
    let p = file::load(&dir().join(format!("{name}.chipdaw"))).unwrap();
    assert!(p.load_report.iter().any(|l| l.contains("VRC6 Saw") && l.contains("can't be kept exactly")), "{:?}", p.load_report);
    // It does differ (otherwise the report would be wrong).
    assert_ne!(render::hash(&render::render(&p, secs(&p))), want_r);
}

#[test]
fn migration_reports_what_it_changed_and_current_files_report_nothing() {
    let p = file::load(&dir().join("volumes_mixed.chipdaw")).unwrap();
    assert!(p.load_report.iter().any(|l| l.contains("Upgraded")), "{:?}", p.load_report);
    let again = file::from_str(&file::to_string(&p)).unwrap();
    assert!(again.load_report.is_empty(), "a version 3 file needs no migration");
}

/// Format v3 files written by the build before Batch 3 (main @ fb2d632, with
/// docs/songs/v3/v3corpus.rs.txt), and their hashes from that build
/// (docs/songs/v3/HASHES.txt). Version 4 adds track names and colours and
/// clip colours; a v3 file must open and sound exactly as before, with the
/// default names (v3 names were informational) and no colours.
const V3: &[(&str, u64, u64)] = &[
    ("scale_test", 0x24a6182d8624e32d, 0x6fbb168621f8eeb7),
    ("demo_band", 0x0c984c632801ef74, 0x5080530f47f64c64),
    ("layout_0", 0xf32404eae4b93fa2, 0xc56c98aeea4a3098),
    ("layout_1", 0x99fb04b277dac084, 0x57720d21669d5fd0),
    ("layout_2", 0x88f50e7e43f9f59e, 0x74293fd8cb504efd),
    ("layout_3", 0xbbfece4fdee90c11, 0x3b7264132f74366f),
    ("layout_4", 0x43621e26f83e83e3, 0x1297fa4fcc0f77b7),
    ("raw_volumes", 0x9ce78c6f9d53c5d2, 0x2f45a89d297f00f8),
    ("saw_rate_50_inexact", 0x1343b5eec2e8d720, 0xcad8f3a659155ded),
    ("showcase", 0x7ec54ac5dd1d130d, 0x075f22c0555c07c8),
    ("volumes_mixed", 0xe1840708a7c60272, 0xd8359071fd0824f9),
];

#[test]
fn version_3_files_render_bit_identically_in_version_4() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v3");
    let mut bad = Vec::new();
    for &(name, want_r, want_e) in V3 {
        let text = std::fs::read_to_string(dir.join(format!("{name}.chipdaw"))).unwrap();
        assert!(text.contains("\"version\": 3"), "{name} is a version 3 file");
        let p = file::from_str(&text).unwrap();
        assert!(p.load_report.is_empty(), "{name}: v3 -> v4 changes nothing audible: {:?}", p.load_report);
        let names = |l: &[ui::state::Lane]| l.iter().map(|x| (x.name.clone(), ui::state::rgb_hex(x.color))).collect::<Vec<_>>();
        assert_eq!(names(&p.arrangement.lanes), names(&ui::state::default_lanes()), "{name}: default names and colours");
        assert!(p.arrangement.clips.iter().all(|c| c.color.is_none()));
        let r = render::hash(&render::render(&p, secs(&p)));
        let e = render::hash(&export::render(&p));
        if (r, e) != (want_r, want_e) {
            bad.push(format!("{name}: render {r:#018x} (want {want_r:#018x}), export {e:#018x} (want {want_e:#018x})"));
        }
        // Saved as the current version and reloaded (format 7 writes every
        // track's colour; clips have none).
        let v4 = file::to_string(&p);
        assert!(v4.contains(&format!("\"version\": {}", file::VERSION)), "{name}");
        assert!(!v4.contains("\"color\": null"), "{name}");
        let again = file::from_str(&v4).unwrap();
        assert_eq!(again.arrangement.lanes, p.arrangement.lanes);
        assert_eq!(render::hash(&export::render(&again)), e, "{name}: v4 round trip");
    }
    assert!(bad.is_empty(), "{}", bad.join("
"));
}

/// A v3 file's lane names were informational: a hand-edited one is ignored
/// (exact migration), while a v4 file's names and colours are kept.
#[test]
fn track_names_and_colours_are_saved_from_version_4() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v3");
    let text = std::fs::read_to_string(dir.join("demo_band.chipdaw")).unwrap();
    let edited = text.replacen("\"name\": \"Pulse 1\"", "\"name\": \"Lead\"", 1);
    assert_ne!(edited, text);
    let p = file::from_str(&edited).unwrap();
    assert_eq!(p.arrangement.lanes[0].name, "Pulse 1", "v3: ignored");
    let mut p = p;
    p.arrangement.lanes[0].name = "Lead".into();
    p.arrangement.lanes[3].color = ui::theme::tokens::Rgba::hex(0x4FA8E0);
    let id = p.arrangement.clips[0].id;
    p.arrangement.clip_mut(id).unwrap().color = Some(ui::theme::tokens::Rgba::hex(0xE0705A));
    let again = file::from_str(&file::to_string(&p)).unwrap();
    assert_eq!(again.arrangement.lanes[0].name, "Lead");
    assert_eq!(ui::state::rgb_hex(again.arrangement.lanes[3].color), 0x4FA8E0);
    assert_eq!(again.arrangement.clip(id).unwrap().color.map(ui::state::rgb_hex), Some(0xE0705A));
}

/// Version 4 files (written by main 23b5da9, the last version 4 build; see
/// docs/songs/v4/HASHES.txt) open in version 5 unchanged: same render and
/// export hashes, no migration notes, and the format they had (48 kHz,
/// 16-bit: the export default then). Saved as version 5 they keep it.
const V4: &[(&str, u64, u64)] = &[
    ("demo_band", 0x0c984c632801ef74, 0x5080530f47f64c64),
    ("layout_0", 0xf32404eae4b93fa2, 0xc56c98aeea4a3098),
    ("layout_1", 0x99fb04b277dac084, 0x57720d21669d5fd0),
    ("layout_2", 0x88f50e7e43f9f59e, 0x74293fd8cb504efd),
    ("layout_3", 0xbbfece4fdee90c11, 0x3b7264132f74366f),
    ("layout_4", 0x43621e26f83e83e3, 0x1297fa4fcc0f77b7),
    ("raw_volumes", 0x9ce78c6f9d53c5d2, 0x2f45a89d297f00f8),
    ("saw_rate_50_inexact", 0x1343b5eec2e8d720, 0xcad8f3a659155ded),
    ("scale_test", 0x24a6182d8624e32d, 0x6fbb168621f8eeb7),
    ("showcase", 0x7ec54ac5dd1d130d, 0x075f22c0555c07c8),
    ("volumes_mixed", 0xe1840708a7c60272, 0xd8359071fd0824f9),
];

#[test]
fn version_4_files_render_bit_identically_in_version_5() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v4");
    let mut bad = Vec::new();
    for &(name, want_r, want_e) in V4 {
        let text = std::fs::read_to_string(dir.join(format!("{name}.chipdaw"))).unwrap();
        assert!(text.contains("\"version\": 4"), "{name} is a version 4 file");
        let p = file::from_str(&text).unwrap();
        assert!(p.load_report.is_empty(), "{name}: v4 -> v5 changes nothing audible: {:?}", p.load_report);
        assert_eq!((p.rate, p.bits), (48_000, 16), "{name}: the export default it had");
        let r = render::hash(&render::render(&p, secs(&p)));
        let e = render::hash(&export::render(&p));
        if (r, e) != (want_r, want_e) {
            bad.push(format!("{name}: render {r:#018x} (want {want_r:#018x}), export {e:#018x} (want {want_e:#018x})"));
        }
        let v5 = file::to_string(&p);
        assert!(v5.contains(&format!("\"version\": {}", file::VERSION)) && v5.contains("\"rate\": 48000") && v5.contains("\"bits\": 16"), "{name}");
        let again = file::from_str(&v5).unwrap();
        assert_eq!((again.rate, again.bits), (48_000, 16));
        assert_eq!(again.arrangement.lanes, p.arrangement.lanes, "{name}: names and colours kept");
        assert_eq!(render::hash(&export::render(&again)), e, "{name}: v5 round trip");
    }
    // The renamed, recoloured track of demo_band survives.
    let demo = file::from_str(&std::fs::read_to_string(dir.join("demo_band.chipdaw")).unwrap()).unwrap();
    assert_eq!(demo.arrangement.lanes[0].name, "Lead");
    assert_eq!(ui::state::rgb_hex(demo.arrangement.lanes[3].color), 0x4FA8E0);
    assert!(bad.is_empty(), "{}", bad.join("
"));
}

#[test]
fn project_format_is_saved_and_checked() {
    let mut p = file::Project::new_legacy();
    assert_eq!((p.rate, p.bits), (file::DEFAULT_PROJECT_RATE, file::DEFAULT_BIT_DEPTH));
    p.rate = 44_100;
    p.bits = 16;
    let text = file::to_string(&p);
    let again = file::from_str(&text).unwrap();
    assert_eq!((again.rate, again.bits), (44_100, 16));
    let odd = text.replacen("\"bits\": 16", "\"bits\": 20", 1);
    assert!(file::from_str(&odd).unwrap_err().contains("bit depth 20"));
}

/// Version 5 files (written by main 8644ff3, the last version 5 build; see
/// docs/songs/v5/HASHES.txt) open in version 6 unchanged: 72 px rows, pan
/// centred, 0 dB (unity: the plain mix runs), the same hashes. Saved as
/// version 6 they write none of the new fields.
const V5: &[(&str, u64, u64)] = V4;

#[test]
fn version_5_files_render_bit_identically_in_version_6() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v5");
    let mut bad = Vec::new();
    for &(name, want_r, want_e) in V5 {
        let text = std::fs::read_to_string(dir.join(format!("{name}.chipdaw"))).unwrap();
        assert!(text.contains("\"version\": 5"), "{name} is a version 5 file");
        let p = file::from_str(&text).unwrap();
        assert!(p.load_report.is_empty(), "{name}: {:?}", p.load_report);
        for l in &p.arrangement.lanes {
            assert_eq!((l.height, l.pan, l.vol_db), (72.0, 0, 0.0), "{name}");
        }
        assert_eq!(p.arrangement.gains(), [1.0; 8]);
        let r = render::hash(&render::render(&p, secs(&p)));
        let e = render::hash(&export::render(&p));
        if (r, e) != (want_r, want_e) {
            bad.push(format!("{name}: render {r:#018x} (want {want_r:#018x}), export {e:#018x} (want {want_e:#018x})"));
        }
        let v6 = file::to_string(&p);
        assert!(v6.contains(&format!("\"version\": {}", file::VERSION)) && !v6.contains("\"height\"") && !v6.contains("\"pan\"") && !v6.contains("\"volume_db\""), "{name}");
        let again = file::from_str(&v6).unwrap();
        assert_eq!(again.arrangement.lanes, p.arrangement.lanes);
        assert_eq!(render::hash(&export::render(&again)), e, "{name}: v6 round trip");
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// Polish 2 §3/§4: heights, pan and volume are saved per track; a track at
/// +6 dB renders louder, at −inf silent, and the others are untouched.
#[test]
fn heights_pan_and_volume_are_saved_from_version_6() {
    let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v5/showcase.chipdaw")).unwrap();
    let mut p = file::from_str(&text).unwrap();
    let base = export::render(&p);
    p.arrangement.lanes[0].height = 144.0;
    p.arrangement.lanes[1].pan = -25;
    p.arrangement.lanes[2].vol_db = f32::NEG_INFINITY;
    p.arrangement.lanes[3].vol_db = -3.5;
    let saved = file::to_string(&p);
    assert!(saved.contains("\"volume_db\": \"-inf\"") && saved.contains("\"pan\": -25") && saved.contains("\"height\": 144.0"), "{saved}");
    let again = file::from_str(&saved).unwrap();
    assert_eq!(again.arrangement.lanes, p.arrangement.lanes);
    // Pan is saved but not heard yet (stereo mixing is Batch 4): only the
    // volumes change the render.
    let mut pan_only = file::from_str(&text).unwrap();
    pan_only.arrangement.lanes[1].pan = -25;
    pan_only.arrangement.lanes[0].height = 144.0;
    assert_eq!(render::hash(&export::render(&pan_only)), render::hash(&base));
    assert_ne!(render::hash(&export::render(&again)), render::hash(&base));
    // Out-of-range heights are clamped to the 4 px steps.
    let odd = saved.replacen("\"height\": 144.0", "\"height\": 1000.0", 1);
    assert_eq!(file::from_str(&odd).unwrap().arrangement.lanes[0].height, 300.0);
}

/// Format v6 files written by the last version 6 build (main 0e669ac, with
/// docs/songs/v6/v6corpus.rs.txt), and their hashes from that build
/// (docs/songs/v6/HASHES.txt). Version 7 (Batch 4a §4.2): tracks are
/// instruments; a v6 file opens with every track locked to its channel and
/// must sound exactly as before.
const V6: &[(&str, u64, u64)] = &[
    ("demo_band", 0x0c984c632801ef74, 0x5080530f47f64c64),
    ("layout_0", 0xf32404eae4b93fa2, 0xc56c98aeea4a3098),
    ("layout_1", 0x99fb04b277dac084, 0x57720d21669d5fd0),
    ("layout_2", 0x88f50e7e43f9f59e, 0x74293fd8cb504efd),
    ("layout_3", 0xbbfece4fdee90c11, 0x3b7264132f74366f),
    ("layout_4", 0x43621e26f83e83e3, 0x1297fa4fcc0f77b7),
    ("raw_volumes", 0x9ce78c6f9d53c5d2, 0x2f45a89d297f00f8),
    ("saw_rate_50_inexact", 0x1343b5eec2e8d720, 0xcad8f3a659155ded),
    ("scale_test", 0x24a6182d8624e32d, 0x6fbb168621f8eeb7),
    ("showcase", 0x7ec54ac5dd1d130d, 0x075f22c0555c07c8),
    ("volumes_mixed", 0xe1840708a7c60272, 0xd8359071fd0824f9),
    ("v6_rows_pan_mute", 0x73345965db691eee, 0xe88cde08b31d97df),
    ("v6_vrc6_unused", 0xa3faf8f55fe930af, 0xb5afeb969a8693ec),
    ("v6_2a03_vrc6_clips", 0xf32404eae4b93fa2, 0xc56c98aeea4a3098),
];

fn v6_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v6")
}

#[test]
fn version_6_files_render_bit_identically_in_version_7() {
    use ui::state::SoundChip;
    let mut bad = Vec::new();
    for &(name, want_r, want_e) in V6 {
        let text = std::fs::read_to_string(v6_dir().join(format!("{name}.chipdaw"))).unwrap();
        assert!(text.contains("\"version\": 6"), "{name} is a version 6 file");
        let p = file::from_str(&text).unwrap();
        assert!(p.load_report.is_empty(), "{name}: v6 -> v7 changes nothing audible: {:?}", p.load_report);
        // Every track with clips (or in the profile) is locked to its own channel.
        let vrc6_clips = p.arrangement.clips.iter().any(|c| c.lane >= 5);
        for (i, l) in p.arrangement.lanes.iter().enumerate() {
            let has_clips = p.arrangement.clips.iter().any(|c| c.lane == i);
            if p.chip.has_channel(i) || has_clips {
                assert_eq!(l.lock, Some(i), "{name}: track {i} locked to its channel");
            } else {
                assert_eq!(l.lock, None, "{name}: an empty VRC6 track under NES is a studio track");
            }
        }
        if text.contains("\"chip\": \"2A03+VRC6\"") {
            assert_eq!(p.chip, if vrc6_clips { SoundChip::WithVrc6 } else { SoundChip::Base2A03 }, "{name}: the profile");
        } else {
            assert_eq!(p.chip, SoundChip::Base2A03, "{name}");
        }
        let r = render::hash(&render::render(&p, secs(&p)));
        let e = render::hash(&export::render(&p));
        if (r, e) != (want_r, want_e) {
            bad.push(format!("{name}: render {r:#018x} (want {want_r:#018x}), export {e:#018x} (want {want_e:#018x})"));
        }
        // Saved as version 7 and reloaded: still the same sound and tracks.
        let v7 = file::to_string(&p);
        assert!(v7.contains(&format!("\"version\": {}", file::VERSION)) && v7.contains("\"tracks\""), "{name}");
        let again = file::from_str(&v7).unwrap();
        assert_eq!(again.arrangement.lanes, p.arrangement.lanes, "{name}");
        assert_eq!((again.chip, again.filter, again.filter_auto), (p.chip, p.filter, p.filter_auto), "{name}");
        assert_eq!((&again.mute, &again.solo), (&p.mute, &p.solo), "{name}");
        assert_eq!(render::hash(&export::render(&again)), e, "{name}: v7 round trip");
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// A 2A03 + VRC6 song with empty VRC6 lanes opens as NES; its automatic
/// filter (the Famicom stage) is kept as an explicit choice so it sounds the
/// same.
#[test]
fn an_unused_vrc6_opens_as_nes_with_its_filter_kept() {
    let p = file::load(&v6_dir().join("v6_vrc6_unused.chipdaw")).unwrap();
    assert_eq!(p.chip, ui::state::SoundChip::Base2A03);
    assert_eq!((p.filter, p.filter_auto), (engine::FilterMode::Famicom, false));
    assert!(p.arrangement.lanes[5..].iter().all(|l| l.lock.is_none()));
    assert!(!ui::tracks::budget(&p.arrangement, p.chip).over());
    // 2A03 with VRC6 clips: locked to channels the profile lacks (over budget).
    let q = file::load(&v6_dir().join("v6_2a03_vrc6_clips.chipdaw")).unwrap();
    assert!(ui::tracks::budget(&q.arrangement, q.chip).over());
    assert!(ui::state::lane_off(q.chip, &q.arrangement, 5));
}

/// A v6 track at -3 dB is locked like every other, so it plays at the chip's
/// level now: the load report says so and unlocking brings -3 dB back.
#[test]
fn a_v6_fader_is_reported_and_restored_on_unlock() {
    let p = file::load(&v6_dir().join("v6_fader_minus3.chipdaw")).unwrap();
    assert!(p.load_report.iter().any(|l| l.contains("Lead") && l.contains("−3.0 dB")), "{:?}", p.load_report);
    assert_eq!((p.arrangement.lanes[0].vol_db, p.arrangement.lanes[0].before_lock), (0.0, Some((-3.0, 0))));
    // It renders as the 0 dB song (the demo) now.
    assert_eq!(render::hash(&export::render(&p)), 0x5080530f47f64c64);
    let mut q = p.clone();
    q.arrangement.unlock_track(0);
    assert_eq!(q.arrangement.lanes[0].vol_db, -3.0);
}

/// Version 7 files (docs/songs/v7, written by the build before Batch 4b,
/// hashes from it): (file, render::render hash, export::render hash).
const V7: &[(&str, u64, u64)] = &[
    ("demo_band", 0x0c984c632801ef74, 0x5080530f47f64c64),
    ("showcase", 0x7ec54ac5dd1d130d, 0x075f22c0555c07c8),
    ("layout_2", 0x88f50e7e43f9f59e, 0x74293fd8cb504efd),
    ("v6_rows_pan_mute", 0x73345965db691eee, 0xe88cde08b31d97df),
    ("v6_vrc6_unused", 0xa3faf8f55fe930af, 0xb5afeb969a8693ec),
    ("scale_test", 0x24a6182d8624e32d, 0x6fbb168621f8eeb7),
    ("v7_studio_demo", 0xfaa80d8bb5a43098, 0xdecd97a4045f332c),
    ("v7_studio_fader_voices", 0xb6fe5013dffff23b, 0xd1ea1ac50678eab3),
    ("v7_studio_showcase", 0x5ee888eb6ce72c4b, 0x3943be55e2b424f7),
];

fn v7_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v7")
}

/// B4B (format v8): version 7 files open with one unison voice, no detune
/// or drift, and every file with no panned studio track renders exactly as
/// the build before Batch 4b did, through the mono path (studio tracks
/// included); saved as version 8 and reloaded, still the same.
#[test]
fn version_7_files_render_bit_identically_in_version_8() {
    let mut bad = Vec::new();
    for &(name, want_r, want_e) in V7 {
        let text = std::fs::read_to_string(v7_dir().join(format!("{name}.chipdaw"))).unwrap();
        assert!(text.contains("\"version\": 7"), "{name} is a version 7 file");
        let p = file::from_str(&text).unwrap();
        assert!(p.arrangement.lanes.iter().all(|l| l.unison == ui::state::Unison::default()), "{name}");
        assert_eq!(ui::tracks::channels(&p.arrangement), 1, "{name}: mono");
        let r = render::hash(&render::render(&p, secs(&p)));
        let e = render::hash(&export::render(&p));
        if (r, e) != (want_r, want_e) {
            bad.push(format!("{name}: render {r:#018x} (want {want_r:#018x}), export {e:#018x} (want {want_e:#018x})"));
        }
        // The two-channel render of a mono project is the mono render.
        let (x, ch) = export::render_channels(&p);
        assert_eq!((ch, render::hash(&x)), (1, e), "{name}: render_channels");
        let v8 = file::to_string(&p);
        assert!(v8.contains("\"version\": 8") && !v8.contains("\"unison\""), "{name}: defaults aren't written");
        let again = file::from_str(&v8).unwrap();
        assert_eq!(again.arrangement.lanes, p.arrangement.lanes, "{name}");
        assert_eq!(render::hash(&export::render(&again)), e, "{name}: v8 round trip");
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// A version 7 studio track at 20L: the pan was saved, not heard; from
/// version 8 it is heard. Centred again it is the version 7 sound exactly.
#[test]
fn a_v7_panned_studio_track_is_heard_now() {
    let p = file::load(&v7_dir().join("v7_studio_panned.chipdaw")).unwrap();
    assert_eq!((p.arrangement.lanes[0].pan, ui::tracks::channels(&p.arrangement)), (-20, 2));
    let (x, ch) = export::render_channels(&p);
    assert_eq!(ch, 2);
    let (l, r): (Vec<f32>, Vec<f32>) = x.as_chunks::<2>().0.iter().map(|f| (f[0], f[1])).unzip();
    assert_ne!(render::hash(&l), render::hash(&r), "panned: the channels differ");
    // Version 7 rendered the panned file as if centred: centred again, it
    // is that sound exactly (the hash from the old build).
    let mut q = p.clone();
    q.arrangement.lanes[0].pan = 0;
    assert_eq!(render::hash(&export::render(&q)), 0x1b1742f7c9b4dd4d);
    // A v8 file keeps unison, detune and drift.
    let mut u = p.clone();
    u.arrangement.lanes[0].unison = ui::state::Unison { voices: 3, detune: 15, drift: 4 };
    let again = file::from_str(&file::to_string(&u)).unwrap();
    assert_eq!(again.arrangement.lanes[0].unison, u.arrangement.lanes[0].unison);
    assert!(file::to_string(&u).contains("\"unison\": 3"));
}
