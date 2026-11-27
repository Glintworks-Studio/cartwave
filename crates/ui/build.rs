//! Records the git commit for Help > About (`CARTWAVE_COMMIT`), and links the
//! Windows manifest the save prompt needs. Falls back to
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
    println!("cargo:rustc-env=CARTWAVE_COMMIT={commit}");
    // Every commit and checkout appends to the HEAD reflog.
    println!("cargo:rerun-if-changed=../../.git/logs/HEAD");
    println!("cargo:rerun-if-changed=build.rs");
    // Windows: the task dialog behind the save prompt needs Common Controls 6
    // in every binary that links rfd (this crate's tests and examples; the
    // app's own build script does the same for the app).
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target == "windows" && env == "msvc" {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
}
