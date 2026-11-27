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

/// `CARTWAVE_FROZEN_METERS=1`, read once: it is asked every frame, and each
/// environment lookup (two since the rename's aliases) costs more than the
/// frame's meter work (Batch 4c §2).
pub fn frozen_meters() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| var("CARTWAVE_FROZEN_METERS").is_some_and(|v| v == "1"))
}

/// `CARTWAVE_PLAYHEAD=<ticks>` (the playhead held while playing), read once.
pub fn fixed_playhead() -> Option<f64> {
    static AT: std::sync::OnceLock<Option<f64>> = std::sync::OnceLock::new();
    *AT.get_or_init(|| var("CARTWAVE_PLAYHEAD").and_then(|v| v.parse().ok()))
}

/// Like [`var`], for paths.
pub fn var_os(name: &str) -> Option<std::ffi::OsString> {
    if enabled() {
        engine::env::var_os(name)
    } else {
        None
    }
}
