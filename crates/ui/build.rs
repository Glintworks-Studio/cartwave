//! Records the git commit for Help > About (`CHIPDAW_COMMIT`). Falls back to
//! "unknown" outside a git checkout (a source tarball), so the build never
//! depends on git being installed.

fn main() {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=CHIPDAW_COMMIT={commit}");
    // Every commit and checkout appends to the HEAD reflog.
    println!("cargo:rerun-if-changed=../../.git/logs/HEAD");
    println!("cargo:rerun-if-changed=build.rs");
}
