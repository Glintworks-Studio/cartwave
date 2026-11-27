//! Dev mode (UX.md §4, F-02): debug keys (F3–F7) and every test-only hook
//! (`CARTWAVE_STRESS`, `CARTWAVE_PROJECT`, `CARTWAVE_FRAMELOG`, …) work only in a
//! debug build or with `CARTWAVE_DEV=1`. A release build without it ignores
//! them, so a stray key or variable can't swap the user's song for a test set.
//!
//! Plain configuration stays available in every build: `CARTWAVE_DATA_DIR`
//! (where settings and recovery live), `CARTWAVE_AUDIO=0` (no audio device) and
//! `CARTWAVE_VSYNC=0` (a measurement knob). They change no document.
//!
//! Every name also works with its pre-rename `CHIPDAW_` prefix (an alias,
//! `engine::env`); dev mode prints a note when one is used.

/// Whether dev mode is on (decided once per process).
pub fn enabled() -> bool {
    engine::env::dev_mode()
}

/// A test hook's environment variable: its value in dev mode, else `None`.
pub fn var(name: &str) -> Option<String> {
    if enabled() {
        engine::env::var(name)
    } else {
        None
    }
}

/// Like [`var`], for paths.
pub fn var_os(name: &str) -> Option<std::ffi::OsString> {
    if enabled() {
        engine::env::var_os(name)
    } else {
        None
    }
}
