//! Writes the generated projects.
//!
//! `cargo run --release -p ui --example make_songs -- [DIR] [--scale-test]`
//! - `DIR/piano_bench.chipdaw` (the piano-roll benchmark; default: %TEMP%)
//! - with `--scale-test`: `DIR/scale_test.chipdaw` in the current format.
//!
//! `docs/songs/scale_test.chipdaw` is *not* rewritten: it is the version-1
//! file the golden and migration tests load (STABILIZE.md §B4). Saving it in
//! the current format would silently turn it into a version-3 file.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = args.iter().find(|a| !a.starts_with("--")).map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&dir).expect("out dir");
    let bench = dir.join("piano_bench.chipdaw");
    ui::file::save(&ui::songs::piano_bench(), &bench).expect("save bench");
    println!("{}", bench.display());
    if args.iter().any(|a| a == "--scale-test") {
        let test = dir.join("scale_test.chipdaw");
        ui::file::save(&ui::songs::scale_test(), &test).expect("save scale test");
        println!("{}", test.display());
    }
}
