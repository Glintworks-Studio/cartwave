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
        // Saved again as version 3 and reloaded: still the same sound.
        let again = file::from_str(&file::to_string(&p)).unwrap();
        assert!(file::to_string(&p).contains("\"version\": 3"));
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
