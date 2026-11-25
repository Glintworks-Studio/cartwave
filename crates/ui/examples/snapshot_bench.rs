//! Snapshot build time vs song size (PIANOROLL.md §7).
//!
//! `cargo run --release -p ui --example snapshot_bench`
//!
//! For 1k, 10k and 50k notes (100 notes per 4-bar clip, 8 lanes) it times,
//! on this thread, what every edit costs today:
//! - `content_key` (run every pump tick),
//! - `song_snapshot` (the full rebuild ENGINE.md's design does per edit),
//! - the same while a clip is being dragged (`editing` set: the model is
//!   cloned and the overlap rule applied first),
//! - an undo commit (`Doc::of` + `History::commit`) after a one-clip edit,
//! - save and load of the project file.
//!
//! Each figure is the median of N runs after a warm-up.

use std::time::{Duration, Instant};

use engine::Params;
use ui::history::{Doc, History};

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn time<R>(runs: usize, mut f: impl FnMut() -> R) -> Duration {
    for _ in 0..3 {
        std::hint::black_box(f());
    }
    median(
        (0..runs)
            .map(|_| {
                let t = Instant::now();
                std::hint::black_box(f());
                t.elapsed()
            })
            .collect(),
    )
}

fn ms(d: Duration) -> String {
    format!("{:.3}", d.as_secs_f64() * 1e3)
}

fn main() {
    println!("| Notes | Clips | content_key | song_snapshot | snapshot while dragging a clip | undo commit | save | load | file size |");
    println!("|---|---|---|---|---|---|---|---|---|");
    for total in [1_000usize, 10_000, 50_000] {
        let p = ui::songs::many_notes(total, 100);
        let mut a = p.arrangement.clone();
        let runs = if total > 20_000 { 21 } else { 51 };
        let key = time(runs, || a.content_key());
        let snap = time(runs, || a.song_snapshot(1));
        let first = a.clips[0].id;
        a.editing = vec![first];
        let drag = time(runs, || a.song_snapshot(1));
        a.editing.clear();
        let mut h = History::new(Doc::of(&a, 150.0, Params::default()));
        let mut i = 0u32;
        let commit = time(runs, || {
            i += 1;
            a.clip_mut(first).unwrap().notes[0].pitch = 60 + (i % 2) as u8;
            h.commit(Doc::of(&a, 150.0, Params::default()))
        });
        let dir = std::env::temp_dir().join("chipdaw-snapbench");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("n{total}.chipdaw"));
        let save = time(5, || ui::file::save(&p, &path).unwrap());
        let load = time(5, || ui::file::load(&path).unwrap());
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        println!(
            "| {total} | {} | {} | {} | {} | {} | {} | {} | {:.1} MB |",
            a.clips.len(),
            ms(key),
            ms(snap),
            ms(drag),
            ms(commit),
            ms(save),
            ms(load),
            size as f64 / 1e6
        );
    }
}
