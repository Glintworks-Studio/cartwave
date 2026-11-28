//! Writes Batch 5's automation fixtures (the audio and performance gates):
//!
//! - `auto32.cartwave`: the demo band (docs/songs/v9/demo_band), tracks 0–3
//!   locked (2A03), tracks 4–7 unlocked to studio tracks, and **32 automated
//!   parameters**, four per track (the first four of each track's registry:
//!   studio tracks' fader and pan among them), each a slow curve over the
//!   song with a breakpoint every two bars, plus the Main gain. The loop is
//!   the whole song, so a ten-minute run keeps playing it.
//! - `auto2000.cartwave`: the same song with one studio lane of **2,000
//!   breakpoints** (track 0, unlocked: its fader, a point every 1/16 for 125
//!   bars), shown in Automation Mode (the drawing scenario).
//! - `*_plain.cartwave`: the same songs and lanes without envelopes (the
//!   performance gate's like-for-like baseline: `main` can't open format 10).
//!
//! usage: cargo run --release -p ui --example auto_fixture -- <out dir>

use ui::automation::{AutoOwner, Envelope, Pt};
use ui::file;

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("out dir"));
    std::fs::create_dir_all(&out).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut p = file::load(&root.join("docs/songs/v9/demo_band.chipdaw")).expect("the demo band");
    for i in 4..p.arrangement.lanes.len().min(8) {
        p.arrangement.unlock_track(i);
    }
    let a = &mut p.arrangement;
    let bar = ui::state::TICKS_PER_BAR;
    let end = p.loop_end_ticks.max(bar * 8);
    let mut n = 0;
    for i in 0..a.lanes.len() {
        let id = a.lanes[i].id;
        let owner = AutoOwner::Track(id);
        let reg = ui::automation::registry_for(&a.lanes, owner).unwrap_or_default();
        for (k, prm) in reg.iter().take(4).enumerate() {
            let lo = if prm.min.is_finite() { prm.min } else { -24.0 };
            let (lo, hi) = (lo + (prm.max - lo) * 0.2, lo + (prm.max - lo) * 0.9);
            let mut pts = Vec::new();
            let mut t = 0;
            let mut j = 0u32;
            while t <= end {
                // A slow wave, its phase per lane.
                let x = ((j + k as u32 * 3 + i as u32) % 8) as f32 / 7.0;
                let v = lo + (hi - lo) * if (j / 8).is_multiple_of(2) { x } else { 1.0 - x };
                pts.push(Pt { t, v, c: if j.is_multiple_of(3) { 0.4 } else { 0.0 } });
                t += 2 * bar;
                j += 1;
            }
            *a.automation.edit(&(owner, prm.id.to_string())) = Envelope::new(pts);
            a.automation.shown.entry(owner).or_default().push(prm.id.to_string());
            n += 1;
        }
    }
    *a.automation.edit(&(AutoOwner::Main, "main.gain".into())) = Envelope::new(vec![Pt::new(0, 0.0), Pt { t: end / 2, v: -6.0, c: 0.3 }, Pt::new(end, 0.0)]);
    p.loop_start_ticks = 0;
    p.loop_on = true;
    file::save(&p, &out.join("auto32.cartwave")).unwrap();
    let mut plain = p.clone();
    plain.arrangement.automation.envs.clear();
    file::save(&plain, &out.join("auto32_plain.cartwave")).unwrap();
    println!("auto32: {n} automated track parameters (+ Main), {} points, loop 0..{}", p.arrangement.automation.point_count(), p.loop_end_ticks);

    // One lane of 2,000 breakpoints.
    let mut q = file::load(&root.join("docs/songs/v9/demo_band.chipdaw")).unwrap();
    q.arrangement.unlock_track(0);
    let id = q.arrangement.lanes[0].id;
    let pts: Vec<Pt> = (0..2000u32).map(|k| Pt::new(k * bar / 16, -20.0 + 18.0 * ((k % 32) as f32 / 31.0))).collect();
    let key = (AutoOwner::Track(id), "track.gain".to_string());
    *q.arrangement.automation.edit(&key) = Envelope::new(pts);
    q.arrangement.automation.shown.insert(AutoOwner::Track(id), vec!["track.gain".into()]);
    file::save(&q, &out.join("auto2000.cartwave")).unwrap();
    println!("auto2000: {} points on track 0's fader", q.arrangement.automation.point_count());
    q.arrangement.automation.envs.clear();
    file::save(&q, &out.join("auto2000_plain.cartwave")).unwrap();
}
