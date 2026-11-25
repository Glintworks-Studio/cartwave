//! Dev mode (UX.md §4, F-02): debug keys (F3–F7) and every test-only hook
//! (`CHIPDAW_STRESS`, `CHIPDAW_PROJECT`, `CHIPDAW_FRAMELOG`, …) work only in a
//! debug build or with `CHIPDAW_DEV=1`. A release build without it ignores
//! them, so a stray key or variable can't swap the user's song for a test set.
//!
//! Plain configuration stays available in every build: `CHIPDAW_DATA_DIR`
//! (where settings and recovery live), `CHIPDAW_AUDIO=0` (no audio device) and
//! `CHIPDAW_VSYNC=0` (a measurement knob). They change no document.

use std::sync::OnceLock;

/// Whether dev mode is on (decided once per process).
pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| cfg!(debug_assertions) || std::env::var("CHIPDAW_DEV").is_ok_and(|v| v == "1"))
}

/// A test hook's environment variable: its value in dev mode, else `None`.
pub fn var(name: &str) -> Option<String> {
    if enabled() {
        std::env::var(name).ok()
    } else {
        None
    }
}

/// Like [`var`], for paths.
pub fn var_os(name: &str) -> Option<std::ffi::OsString> {
    if enabled() {
        std::env::var_os(name)
    } else {
        None
    }
}
