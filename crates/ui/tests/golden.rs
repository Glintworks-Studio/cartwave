//! Golden render test (PIANOROLL.md §9): the checked-in test song renders to
//! the same audio every time. The engine is deterministic across platforms
//! (ENGINE-macos.md §1), so the hash is the same on Windows and macOS.
//!
//! If a deliberate engine or song change alters the sound, regenerate the
//! song (`cargo run --release -p ui --example make_songs`) if needed, listen
//! to it, and update `EXPECTED` with the hash this test prints.

const EXPECTED: u64 = 0xaaac_a2df_8157_ca6a;

#[test]
fn scale_test_renders_to_the_golden_hash() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/scale_test.chipdaw");
    let p = ui::file::load(&path).expect("docs/songs/scale_test.chipdaw loads");
    // The whole song: 8 bars at 150 BPM = 12.8 s.
    let secs = p.loop_end_ticks as f64 / ui::state::PPQ as f64 * 60.0 / p.bpm as f64;
    let a = ui::render::render(&p, secs);
    let b = ui::render::render(&p, secs);
    assert_eq!(ui::render::hash(&a), ui::render::hash(&b), "two renders differ");
    let peak = a.iter().fold(0f32, |m, v| m.max(v.abs()));
    assert!(peak > 0.05, "silent render (peak {peak})");
    // Every channel sounds on its own (solo each lane for the first 4 bars).
    for lane in 0..8 {
        let mut solo = p.clone();
        solo.solo = ui::tracks::TrackSet(vec![p.arrangement.lanes[lane].id]);
        let x = ui::render::render(&solo, secs / 2.0);
        let peak = x.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.01, "lane {lane} is silent (peak {peak})");
    }
    let h = ui::render::hash(&a);
    assert_eq!(h, EXPECTED, "render hash {h:#018x} (peak {peak:.3}, {} samples)", a.len());
}
