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
        assert_eq!(p.arrangement.lanes, ui::state::default_lanes(), "{name}: default names and colours");
        assert!(p.arrangement.clips.iter().all(|c| c.color.is_none()));
        let r = render::hash(&render::render(&p, secs(&p)));
        let e = render::hash(&export::render(&p));
        if (r, e) != (want_r, want_e) {
            bad.push(format!("{name}: render {r:#018x} (want {want_r:#018x}), export {e:#018x} (want {want_e:#018x})"));
        }
        // Saved as the current version (no colour fields for default colours) and reloaded.
        let v4 = file::to_string(&p);
        assert!(v4.contains(&format!("\"version\": {}", file::VERSION)) && !v4.contains("\"color\""), "{name}");
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
        assert!(v5.contains("\"version\": 5") && v5.contains("\"rate\": 48000") && v5.contains("\"bits\": 16"), "{name}");
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
    let mut p = file::Project::new_empty();
    assert_eq!((p.rate, p.bits), (file::DEFAULT_PROJECT_RATE, file::DEFAULT_BIT_DEPTH));
    p.rate = 44_100;
    p.bits = 16;
    let text = file::to_string(&p);
    let again = file::from_str(&text).unwrap();
    assert_eq!((again.rate, again.bits), (44_100, 16));
    let odd = text.replacen("\"bits\": 16", "\"bits\": 20", 1);
    assert!(file::from_str(&odd).unwrap_err().contains("bit depth 20"));
}
