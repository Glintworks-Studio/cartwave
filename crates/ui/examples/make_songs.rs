//! Writes the generated projects.
//!
//! `cargo run --release -p ui --example make_songs -- [DIR]`
//! - `docs/songs/scale_test.chipdaw` (the checked-in test song)
//! - `DIR/piano_bench.chipdaw` (the piano-roll benchmark; default: %TEMP%)

fn main() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let songs = repo.join("docs/songs");
    std::fs::create_dir_all(&songs).expect("docs/songs");
    let test = songs.join("scale_test.chipdaw");
    ui::file::save(&ui::songs::scale_test(), &test).expect("save scale test");
    println!("{}", test.display());
    let dir = std::env::args().nth(1).map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let bench = dir.join("piano_bench.chipdaw");
    ui::file::save(&ui::songs::piano_bench(), &bench).expect("save bench");
    println!("{}", bench.display());
}
