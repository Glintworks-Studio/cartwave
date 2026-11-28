//! Prints the render and export hashes of project files, one line each, for
//! comparing two builds (RENAME.md gate 2: the `.chipdaw` corpus opens and
//! renders identically before and after the rename).
//!
//! `cargo run --release -p ui --example corpus_hashes -- <list.txt>`
//! where `list.txt` holds one project path per line. Output:
//! `path<TAB>render hash<TAB>export hash<TAB>channels<TAB>console hash<TAB>studio
//! tracks` (or `path<TAB>error<TAB>message`); the console hash is the export
//! with every studio track muted (Batch 5).

fn main() {
    let list = std::env::args().nth(1).expect("usage: corpus_hashes <list.txt>");
    let text = std::fs::read_to_string(&list).expect("read the list");
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        match ui::file::load(std::path::Path::new(line)) {
            Ok(p) => {
                let secs = p.loop_end_ticks as f64 / ui::state::PPQ as f64 * 60.0 / p.bpm as f64 + 1.0;
                let r = ui::render::hash(&ui::render::render(&p, secs.min(60.0)));
                let (x, ch) = ui::export::render_channels(&p);
                // Batch 5 A3: the locked tracks alone (every studio track
                // muted), which must match between builds even where a studio
                // track's sound changed.
                let mut q = p.clone();
                for l in &q.arrangement.lanes {
                    if l.lock.is_none() {
                        q.mute.set(l.id, true);
                        q.solo.set(l.id, false);
                    }
                }
                let c = ui::render::hash(&ui::export::render_channels(&q).0);
                let studio = p.arrangement.lanes.iter().filter(|l| l.lock.is_none()).count();
                println!("{line}\t{r:#018x}\t{:#018x}\t{ch}\t{c:#018x}\t{studio}", ui::render::hash(&x));
            }
            Err(e) => println!("{line}\terror\t{e}"),
        }
    }
}
