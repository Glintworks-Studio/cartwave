//! Batch 4a §9.5: the 64-studio-track stress project (docs/batch4a/songs/stress64.chipdaw,
//! made by docs/batch4a/tools/make_stress64.py) loads, every track is a studio track, its
//! pools hold 8-note chords without stealing, and an offline render through the engine's
//! `Mixdown` keeps up with real time by a wide margin (the live engine runs the same code).
use std::time::Instant;
use ui::{file, tracks};

#[test]
fn stress64_renders_512_voices_faster_than_real_time() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/batch4a/songs/stress64.chipdaw");
    let p = file::load(&path).expect("loads");
    assert_eq!(p.arrangement.lanes.len(), 64);
    assert!(p.arrangement.lanes.iter().all(|l| l.lock.is_none()));
    let shape = p.arrangement.rack_shape();
    assert_eq!(shape.len(), 64);
    assert!(shape.iter().all(|s| s.pool >= 8), "{shape:?}");
    let heard = tracks::audible(&p.arrangement, &p.mute, &p.solo);
    let (mut m, snap) = tracks::offline_mixdown(&p.arrangement, p.chip, p.filter, p.bpm as f64, 48_000, &heard, 0.0, true);
    m.player.play();
    let secs = 4.0;
    let mut out = vec![0f32; (secs * 48_000.0) as usize];
    let t0 = Instant::now();
    for c in out.chunks_mut(256) {
        m.render(&snap, c);
    }
    let took = t0.elapsed().as_secs_f64();
    let sounding: u32 = m.rack.tracks.iter().map(|t| t.ctl.sounding.load(std::sync::atomic::Ordering::Relaxed) as u32).sum();
    let steals: u32 = m.rack.tracks.iter().map(|t| t.ctl.steals.load(std::sync::atomic::Ordering::Relaxed)).sum();
    println!("stress64: {secs} s of 64 tracks x 8 voices rendered in {took:.3} s ({:.1}% of real time); {sounding} voices sounding; {steals} steals", took / secs * 100.0);
    assert_eq!(steals, 0, "the pools are big enough");
    assert!(sounding >= 500, "{sounding}");
    assert!(out.iter().any(|v| *v != 0.0));
    assert!(took < secs * 0.5, "{took:.3} s for {secs} s");
}
