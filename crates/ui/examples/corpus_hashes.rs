//! Prints the render and export hashes of project files, one line each, for
//! comparing two builds (RENAME.md gate 2: the `.chipdaw` corpus opens and
//! renders identically before and after the rename).
//!
//! `cargo run --release -p ui --example corpus_hashes -- <list.txt>`
//! where `list.txt` holds one project path per line. Output:
//! `path<TAB>render hash<TAB>export hash` (or `path<TAB>error<TAB>message`).

fn main() {
    let list = std::env::args().nth(1).expect("usage: corpus_hashes <list.txt>");
    let text = std::fs::read_to_string(&list).expect("read the list");
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        match ui::file::load(std::path::Path::new(line)) {
            Ok(p) => {
                let secs = p.loop_end_ticks as f64 / ui::state::PPQ as f64 * 60.0 / p.bpm as f64 + 1.0;
                let r = ui::render::hash(&ui::render::render(&p, secs.min(60.0)));
                let (x, ch) = ui::export::render_channels(&p);
                println!("{line}\t{r:#018x}\t{:#018x}\t{ch}", ui::render::hash(&x));
            }
            Err(e) => println!("{line}\terror\t{e}"),
        }
    }
}
