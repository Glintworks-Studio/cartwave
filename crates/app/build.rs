//! Windows: embed a manifest dependency on Common Controls 6, which the
//! "Save / Don't Save / Cancel" task dialog (rfd's `common-controls-v6`
//! feature) needs. Same lines as `crates/ui/build.rs` (for its tests and
//! examples, which link rfd too).

fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target == "windows" && env == "msvc" {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
